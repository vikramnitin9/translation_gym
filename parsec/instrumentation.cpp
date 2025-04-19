#include "instrumentation.h"

using namespace llvm;
using json = nlohmann::json;

bool compareFilenames(std::string filename1, std::string filename2) {
	std::string baseName1 = llvm::sys::path::filename(filename1);
	std::string baseName2 = llvm::sys::path::filename(filename2);
	return baseName1 == baseName2;
}

void addInstrumentation(Module &M, std::unordered_set<json> jsonData) {
	// Add instrumentation code to the module
	LLVMContext &Context = M.getContext();
	IRBuilder<> Builder(Context);
	
	// Declare standard library functions
	FunctionCallee Getenv = M.getOrInsertFunction(
        "getenv", FunctionType::get(PointerType::get(Type::getInt8Ty(Context), 0),
                                    {PointerType::get(Type::getInt8Ty(Context), 0)}, false));

	FunctionCallee Snprintf = M.getOrInsertFunction(
		"snprintf", FunctionType::get(IntegerType::getInt32Ty(Context),
									{PointerType::get(Type::getInt8Ty(Context), 0),
										IntegerType::getInt64Ty(Context),
										PointerType::get(Type::getInt8Ty(Context), 0),
										PointerType::get(Type::getInt8Ty(Context), 0)},
									true));

	FunctionCallee Fopen = M.getOrInsertFunction(
		"fopen", FunctionType::get(PointerType::get(Type::getInt8Ty(Context), 0),
									{PointerType::get(Type::getInt8Ty(Context), 0),
									PointerType::get(Type::getInt8Ty(Context), 0)}, false));

	FunctionCallee Fprintf = M.getOrInsertFunction(
		"fprintf", FunctionType::get(IntegerType::getInt32Ty(Context),
									{PointerType::get(Type::getInt8Ty(Context), 0),
									PointerType::get(Type::getInt8Ty(Context), 0)}, true));

	FunctionCallee Fclose = M.getOrInsertFunction(
		"fclose", FunctionType::get(IntegerType::getInt32Ty(Context),
									{PointerType::get(Type::getInt8Ty(Context), 0)}, false));

	FunctionCallee Strcpy = M.getOrInsertFunction(
		"strcpy", FunctionType::get(Type::getInt8PtrTy(Context),
									{Type::getInt8PtrTy(Context),
									 Type::getInt8PtrTy(Context)}, false));

	FunctionCallee Strcat = M.getOrInsertFunction(
		"strcat", FunctionType::get(Type::getInt8PtrTy(Context),
									{Type::getInt8PtrTy(Context),
									 Type::getInt8PtrTy(Context)}, false));

	FunctionCallee Strlen = M.getOrInsertFunction(
		"strlen", llvm::FunctionType::get(llvm::Type::getInt64Ty(Context),
    								{llvm::Type::getInt8PtrTy(Context)}, false));

	// Iterate over all functions in the module
	for (Function &F : M) {
		// Skip functions that are not defined in the module
		if (F.isDeclaration()) continue;

		llvm::DISubprogram *SubProg = F.getSubprogram();
		if (!SubProg){
			continue; // Skip functions without debug info
		}
		std::vector<std::string> argTypes = {};
		std::vector<std::string> argNames = {};
		std::string retType = "";
		bool found = false;
		for (auto &entry : jsonData) {
			if (entry["name"] == F.getName() &&
					compareFilenames(entry["filename"], SubProg->getFilename())) {
				argTypes = entry["argTypes"];
				argNames = entry["argNames"];
				retType = entry["returnType"];
				found = true;
				break;
			}
		}
		if (!found) continue;

		Builder.SetInsertPoint(&*F.getEntryBlock().getFirstInsertionPt());

		// Get environment variable
		Value *EnvVarName = Builder.CreateGlobalStringPtr("INSTRUMENTATION_PATH");
		Value *DirPath = Builder.CreateCall(Getenv, {EnvVarName});
		Value *DefaultPath = Builder.CreateGlobalStringPtr("./instrumented.json");

		// Check if INSTRUMENTATION_PATH is set
		Value *IsEnvSet = Builder.CreateICmpNE(DirPath, ConstantPointerNull::get(cast<PointerType>(DirPath->getType())));

		// Allocate buffer for full path
		Value *FullPathBuffer = Builder.CreateAlloca(ArrayType::get(Type::getInt8Ty(Context), 512));

		// Convert FullPathBuffer to i8* (char*)
		Value *FullPathPtr = Builder.CreateBitCast(FullPathBuffer, Type::getInt8PtrTy(Context));

		// Format string: "%s/instrumented.json"
		Value *FormatStr = Builder.CreateGlobalStringPtr("%s/instrumented.json");

		// Concatenate path using snprintf
		Builder.CreateCall(Snprintf, {FullPathPtr, ConstantInt::get(Type::getInt64Ty(Context), 512),
																	FormatStr, DirPath});

		// Use CreateSelect with correctly typed pointers
		Value *FinalFilePath = Builder.CreateSelect(IsEnvSet, FullPathPtr, DefaultPath);
		
		// Open file in append mode
		Value *Mode = Builder.CreateGlobalStringPtr("a");
		Value *FilePtr = Builder.CreateCall(Fopen, {FinalFilePath, Mode});

		// Allocate space for the format string
		Value *FmtStrBuffer = Builder.CreateAlloca(ArrayType::get(Type::getInt8Ty(Context), 512));
		Value *FmtStrPtr = Builder.CreateBitCast(FmtStrBuffer, Type::getInt8PtrTy(Context));
		
		Value *InitialStr = Builder.CreateGlobalStringPtr("{ \"name\": \"" + F.getName().str() + "\", \"args\": {");
		Builder.CreateCall(Strcpy, {FmtStrPtr, InitialStr});
		// std::string jsonStr = "{ \"name\": \"" + F.getName().str() + "\", \"args\": [";
		std::vector<Value *> PrintArgs;

		int i = 0;
		// Enumerate the arguments
		for (auto &Arg : F.args()) {
			std::string ArgName;
			std::string ArgTypeStr;
			if (Arg.hasStructRetAttr()) {
				ArgName = "<return_val>";
				ArgTypeStr = retType;
			}
			else {
				if (i >= argNames.size()) {
					llvm::errs() << "Error: Argument index out of bounds\n";
					continue;
				}
				ArgName = argNames[i];
				ArgTypeStr = argTypes[i];
				i++;
			}

			Type *ArgType = Arg.getType();
	
			if (ArgType->isIntegerTy()) {
					Value *Fmt = Builder.CreateGlobalStringPtr("\"" + ArgName + "\"" + " : \"%d\", ");
					Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
					PrintArgs.push_back(&Arg);
			} else if (ArgType->isFloatingPointTy()) {
					Value *Fmt = Builder.CreateGlobalStringPtr("\"" + ArgName + "\"" + " : \"%f\", ");
					Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
					Value *DoubleVal = Builder.CreateFPExt(&Arg, Type::getDoubleTy(Context));
					PrintArgs.push_back(DoubleVal);
			} else if (ArgType->isPointerTy()) {
					Type *ElemType = ArgType->getPointerElementType();
					
					if (ElemType->isIntegerTy(8) &&
					 		// LLVM uses i8* for void* also, so we need this extra check
							(ArgTypeStr == "char *" || ArgTypeStr == "const char *" || ArgTypeStr == "unsigned char *")) {
						// char* arg — check if null
						Value *IsNull = Builder.CreateICmpEQ(&Arg, ConstantPointerNull::get(cast<PointerType>(Arg.getType())));

						// Build "%s" or "\"<null>\""
						Value *RealFmt = Builder.CreateGlobalStringPtr("\"" + ArgName + "\"" + " : \"%.100s\", ");
						Value *NullFmt = Builder.CreateGlobalStringPtr("\"" + ArgName + "\"" + " : \"%p\", ");
						Value *SelectedFmt = Builder.CreateSelect(IsNull, NullFmt, RealFmt);

						// Append to format string (strcat or manual copy)
						Builder.CreateCall(Strcat, {FmtStrPtr, SelectedFmt});
						PrintArgs.push_back(&Arg);
					} else {
							Value *Fmt = Builder.CreateGlobalStringPtr("\"" + ArgName + "\"" + " : \"%p\", ");
							Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
							PrintArgs.push_back(&Arg);
					}
			} else {
					Value *Fmt = Builder.CreateGlobalStringPtr("\"" + ArgName + "\"" + " : \"<unsupported>\", ");
					Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
			}
		}
		
		// if (!PrintArgs.empty()) jsonStr.pop_back(), jsonStr.pop_back();
		// jsonStr += "], \"return\": [";
		// Trim the last ", "
		if (!PrintArgs.empty()) {
			Value *StrLen = Builder.CreateCall(Strlen, {FmtStrPtr});
			Value *Two = ConstantInt::get(StrLen->getType(), 2);
			Value *Idx = Builder.CreateSub(StrLen, Two);
			Value *CommaPtr = Builder.CreateGEP(Type::getInt8Ty(Context), FmtStrPtr, Idx);
			Builder.CreateStore(ConstantInt::get(Type::getInt8Ty(Context), 0), CommaPtr);
		}

		Builder.CreateCall(Strcat, {FmtStrPtr, Builder.CreateGlobalStringPtr("}, \"return\": [")});

		// // Insert before return instructions
		for (auto &BB : F) {
			if (ReturnInst *Ret = dyn_cast<ReturnInst>(BB.getTerminator())) {
				Builder.SetInsertPoint(Ret);
				Value *RetVal = Ret->getReturnValue();
				if (RetVal) {
					if (F.getReturnType()->isIntegerTy()) {
						Value *Fmt = Builder.CreateGlobalStringPtr("\"%d\"");
						Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
						PrintArgs.push_back(RetVal);
					} else if (F.getReturnType()->isFloatingPointTy()) {
						Value *Fmt = Builder.CreateGlobalStringPtr("\"%f\"");
						Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
						Value *DoubleVal = Builder.CreateFPExt(RetVal, Type::getDoubleTy(Context));
						PrintArgs.push_back(DoubleVal);
					} else if (F.getReturnType()->isPointerTy()) {
						Type *ElemType = F.getReturnType()->getPointerElementType();
						
						if (ElemType->isIntegerTy(8) && retType == "char *") { // char* (i8*)
							// char* arg — check if null
							Value *IsNull = Builder.CreateICmpEQ(RetVal, ConstantPointerNull::get(cast<PointerType>(F.getReturnType())));
	
							// Build "%s" or "\"<null>\""
							Value *RealFmt = Builder.CreateGlobalStringPtr("\"%.100s\"");
							Value *NullFmt = Builder.CreateGlobalStringPtr("\"%p\"");
							Value *SelectedFmt = Builder.CreateSelect(IsNull, NullFmt, RealFmt);
	
							// Append to format string (strcat or manual copy)
							Builder.CreateCall(Strcat, {FmtStrPtr, SelectedFmt});
							PrintArgs.push_back(RetVal);
						} else {
								Value *Fmt = Builder.CreateGlobalStringPtr("\"%p\"");
								Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
								PrintArgs.push_back(RetVal);
						}
					} else {
						Value *Fmt = Builder.CreateGlobalStringPtr("\"<unsupported>\"");
						Builder.CreateCall(Strcat, {FmtStrPtr, Fmt});
					}
				}
				Builder.CreateCall(Strcat, {FmtStrPtr, Builder.CreateGlobalStringPtr("] },\n")});

				// Write to file
				// Constant *JsonFormatStr = Builder.CreateGlobalStringPtr(jsonStr);
				// std::vector<Value *> FprintfArgs = {FilePtr, JsonFormatStr};
				std::vector<Value *> FprintfArgs = {FilePtr, FmtStrPtr};
				FprintfArgs.insert(FprintfArgs.end(), PrintArgs.begin(), PrintArgs.end());
				Builder.CreateCall(Fprintf, FprintfArgs);
			}
		}

		// Close file
		Builder.CreateCall(Fclose, {FilePtr});
	}
}