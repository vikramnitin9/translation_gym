#include "clang/Tooling/CommonOptionsParser.h"
#include "clang/Tooling/Tooling.h"

#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/IR/GlobalValue.h"
#include "llvm/Analysis/CallGraph.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Passes/PassPlugin.h"
#include "llvm/Support/VirtualFileSystem.h"
#include "llvm/Support/ToolOutputFile.h"
#include "llvm/Support/MemoryBuffer.h"
#include "llvm/Bitcode/BitcodeWriter.h"

#include "llvm/Target/TargetMachine.h"
#include "llvm/Support/FileSystem.h"
#include "llvm/Support/TargetRegistry.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/Support/Host.h"
#include "llvm/Support/raw_ostream.h"

#include "nlohmann/json.hpp"

#include <iostream>
#include <fstream>

#include "FunctionVisitor.h"
#include "FunctionVisitorConsumer.h"
#include "FunctionVisitAction.h"
#include "ToolActionWrapper.h"
#include "instrumentation.h"
#include "helpers.h"

using namespace clang;
using namespace clang::tooling;
using namespace clang::driver;
using json = nlohmann::json;

static llvm::cl::OptionCategory MyCategory("");

// Custom command line options
static llvm::cl::opt<bool> renameMain(
    "rename-main",
    llvm::cl::desc("Rename main as main_0"),
    llvm::cl::value_desc("bool"),
    llvm::cl::cat(MyCategory),
    llvm::cl::init(true));

static llvm::cl::opt<bool> addInstr(
	"add-instr",
	llvm::cl::desc("Add instrumentation to the module"),
	llvm::cl::value_desc("bool"),
	llvm::cl::cat(MyCategory),
	llvm::cl::init(false));

int main(int argc, const char **argv) {
	auto expectedParser = CommonOptionsParser::create(argc, argv, MyCategory, llvm::cl::ZeroOrMore, "parsec <source0> [... <sourceN>]");
	if (!expectedParser) {
		llvm::errs() << expectedParser.takeError();
		return 1;
	}
	
	// Create a virtual file system
	IntrusiveRefCntPtr<llvm::vfs::InMemoryFileSystem> memFS(new llvm::vfs::InMemoryFileSystem);
	memFS->addFile("instrumentation_helpers.c", 0, llvm::MemoryBuffer::getMemBuffer(cpp_source));

	// Add the virtual file to the tool’s processing list
	CommonOptionsParser& optionsParser = expectedParser.get();
	std::vector<std::string> allSources = optionsParser.getSourcePathList();
	allSources.push_back("instrumentation_helpers.c");  // Include the virtual source file

	// Create a VFS-aware FileManager
	IntrusiveRefCntPtr<FileManager> fileManager = new FileManager(FileSystemOptions(), memFS);
	ClangTool tool(optionsParser.getCompilations(),
				allSources,
				std::make_shared<PCHContainerOperations>(),
				llvm::vfs::getRealFileSystem(),
				fileManager);

	tool.mapVirtualFile("instrumentation_helpers.c", cpp_source);

	VisitorConfig config = {
		.renameMainFunction = renameMain,
		.excludedFunctions = {
			"json_escape"
		},
		.excludedFiles = {
			"instrumentation_helpers.c"
		}
	};
	ToolActionWrapper actionWrapper(new FunctionVisitAction(config));
	tool.run(&actionWrapper);
	
	FunctionVisitAction *action = static_cast<FunctionVisitAction*>(actionWrapper.getAction());
	std::unique_ptr<llvm::Module> M = action->getModule();
	json jsonData = action->getData();

	// Rename main as main_0 to avoid clash with main in Rust
	llvm::Function *MainFunc = M.get()->getFunction("main");
	if (MainFunc && renameMain) {
		MainFunc->setName("main_0");
		if (llvm::DISubprogram *SP = MainFunc->getSubprogram()) {
			llvm::LLVMContext &Ctx = M.get()->getContext();
			SP->replaceOperandWith(2, llvm::MDString::get(Ctx, "main_0"));
		}
		std::cout << "Renamed function: main -> main_0\n";
	}
	// Run a CallGraphAnalysis pass on M
	llvm::PassBuilder PB;
	llvm::ModuleAnalysisManager MAM;
	PB.registerModuleAnalyses(MAM);
	llvm::CallGraph &CG = MAM.getResult<llvm::CallGraphAnalysis>(*M);

	for (llvm::Function &F : M->functions()) {
		if (F.isDeclaration()){
			continue; // Skip declarations without a body
		}
		// Get the subprogram that contains the function
		llvm::DISubprogram *SubProg = F.getSubprogram();
		if (!SubProg){
			continue; // Skip functions without debug info
		}
		// Check if the filename contains "instrumentation_helpers.c"
		std::string filename = SubProg->getFilename();
		if (filename.find("instrumentation_helpers.c") != std::string::npos) {
			continue; // These are instrumentation helper functions added by us
		}
		const llvm::Function *constFunctionPtr = &F;
		// Store all the functions that are called by the current function
		std::set<std::string> seenFunctions;
		json calledFunctions = json::array();
		for (auto &I : *CG[constFunctionPtr]) {
			if (I.second && I.second->getFunction()) {
				// if (!I.second->getFunction()->isDeclaration()) {
					std::string funcName = I.second->getFunction()->getName().str();
					if (seenFunctions.insert(funcName).second) {
						calledFunctions.push_back({{"name", funcName}});
					}
				// }
			}
		}
		for (auto &entry : jsonData["functions"]) {
			if (entry["name"] == F.getName() &&
				compareFilenames(entry["filename"], SubProg->getFilename())) {
				entry["calledFunctions"] = calledFunctions;
				break;
			}
		}
	}

	// This part is necessary for Rust to be able to link against static functions
	for (llvm::Function &F : M->functions()) {
		if (F.getName() == "write_error") {
			F.setLinkage(llvm::GlobalValue::ExternalLinkage);
			F.setVisibility(llvm::GlobalValue::DefaultVisibility);
		}
	}

	// Write this jsonData to a file analysis.json
	std::ofstream outFile("analysis.json");
	if (outFile.is_open()) {
		outFile << std::setw(4) << jsonData << std::endl;
		outFile.close();
		std::cout << "Data written to analysis.json\n";
	} else {
		std::cerr << "Unable to open file analysis.json\n";
		return 1;
	}
	
	if (addInstr) {
		std::cout << "Adding instrumentation to the module\n";
		addInstrumentation(*M, jsonData);
	}

	std::error_code EC;
	llvm::legacy::PassManager PM;
	std::string ErrorMessage;  // Use std::string for the error message

	// // Write the module to an .ll file
	// llvm::raw_fd_ostream IRFile("instrumented.ll", EC);
	// M->print(IRFile, nullptr);

    std::string TargetTriple = llvm::sys::getDefaultTargetTriple();

    llvm::raw_fd_ostream ObjFile("foo.o", EC, llvm::sys::fs::OF_None);

    // Initialize all target-related information
    llvm::InitializeAllTargetInfos();           // Initialize all target infos
    llvm::InitializeAllTargets();               // Initialize all targets
    llvm::InitializeAllTargetMCs();             // Initialize all target machine code generation
    llvm::InitializeAllAsmParsers();            // Initialize all assembly parsers
    llvm::InitializeAllAsmPrinters();           // Initialize all assembly printers

    // Look up the target based on the triple string
    const llvm::Target* Target = llvm::TargetRegistry::lookupTarget(TargetTriple, ErrorMessage);

    if (!Target) {
        llvm::errs() << "Error: " << ErrorMessage << "\n";
        return 1;
    }

    // Now proceed with creating the TargetMachine and generating the object file as usual
	auto RM = llvm::Optional<llvm::Reloc::Model>(llvm::Reloc::PIC_);
    llvm::TargetOptions Options;
    auto TM = Target->createTargetMachine(TargetTriple, "generic", "", Options, RM);

    M->setDataLayout(TM->createDataLayout());
    if (TM->addPassesToEmitFile(PM, ObjFile, nullptr, llvm::CGFT_ObjectFile)) {
        llvm::errs() << "TargetMachine can't emit object file!\n";
    }

    PM.run(*M);
    ObjFile.close();
	// std::cout << "Instrumented object file written to foo.o\n";

	 // Invoke `ar` to create a static library
	 int result = std::system("ar rcs libfoo.a foo.o");
	 if (result != 0) {
		 llvm::errs() << "Failed to create static library\n";
		 return 1;
	 }
	 std::cout << "Static library created: libfoo.a\n";
	 // Clean up the object file
	 EC = llvm::sys::fs::remove("foo.o", false);
	 if (EC) {
		 llvm::errs() << "Failed to remove object file: " << EC.message() << "\n";
		 return 1;
	 }
	 return 0;
}