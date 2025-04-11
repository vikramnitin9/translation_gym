#include "instrumentation.h"

using namespace llvm;

void addInstrumentation(Module &M) {
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

	// Iterate over all functions in the module
	for (Function &F : M) {
		// Skip functions that are not defined in the module
		if (F.isDeclaration()) continue;

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

		std::string jsonStr = "{ \"name\": \"" + F.getName().str() + "\", \"args\": [";
		std::vector<Value *> PrintArgs;
		
		for (auto &Arg : F.args()) {
			Type *ArgType = Arg.getType();
	
			if (ArgType->isIntegerTy()) {
					jsonStr += "\"%d\", ";
					PrintArgs.push_back(&Arg);
			} else if (ArgType->isFloatingPointTy()) {
					jsonStr += "\"%f\", ";
					Value *DoubleVal = Builder.CreateFPExt(&Arg, Type::getDoubleTy(Context));
					PrintArgs.push_back(DoubleVal);
			} else if (ArgType->isPointerTy()) {
					Type *ElemType = ArgType->getPointerElementType();
					
					if (ElemType->isIntegerTy(8)) { // char* (i8*)
							jsonStr += "\"%.100s\", ";  // Limit to 100 chars
							PrintArgs.push_back(&Arg);
					} else {
							jsonStr += "\"%p\", ";
							PrintArgs.push_back(&Arg);
					}
			} else {
					jsonStr += "\"<unsupported>\", ";
			}
		}
		
		if (!PrintArgs.empty()) jsonStr.pop_back(), jsonStr.pop_back();
		jsonStr += "], \"return\": [";

		// Insert before return instructions
		for (auto &BB : F) {
			if (ReturnInst *Ret = dyn_cast<ReturnInst>(BB.getTerminator())) {
				Builder.SetInsertPoint(Ret);
				Value *RetVal = Ret->getReturnValue();
				if (RetVal) {
					if (F.getReturnType()->isIntegerTy()) {
						jsonStr += "\"%d\"";
						PrintArgs.push_back(RetVal);
					} else if (F.getReturnType()->isFloatingPointTy()) {
						jsonStr += "\"%f\"";
						Value *DoubleVal = Builder.CreateFPExt(RetVal, Type::getDoubleTy(Context));
						PrintArgs.push_back(DoubleVal);
					} else if (F.getReturnType()->isPointerTy()) {
                        Type *ElemType = F.getReturnType()->getPointerElementType();
					
                        if (ElemType->isIntegerTy(8)) { // char* (i8*)
                                jsonStr += "\"%.100s\", ";  // Limit to 100 chars
                                PrintArgs.push_back(RetVal);
                        } else {
                                jsonStr += "\"%p\", ";
                                PrintArgs.push_back(RetVal);
                        }
					} else {
						jsonStr += "\"<unsupported>\"";
					}
				}
				jsonStr += "] },\n";

				// Write to file
				Constant *JsonFormatStr = Builder.CreateGlobalStringPtr(jsonStr);
				std::vector<Value *> FprintfArgs = {FilePtr, JsonFormatStr};
				FprintfArgs.insert(FprintfArgs.end(), PrintArgs.begin(), PrintArgs.end());
				Builder.CreateCall(Fprintf, FprintfArgs);
			}
		}

		// Close file
		Builder.CreateCall(Fclose, {FilePtr});
	}
}