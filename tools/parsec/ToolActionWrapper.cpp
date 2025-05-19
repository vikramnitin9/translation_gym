#include "ToolActionWrapper.h"

using namespace clang;
using json = nlohmann::json;
        
// Ideally this function should never be called
std::unique_ptr<FrontendAction> ToolActionWrapper::create() {
    std::cerr << "create() should not be called in ToolActionWrapper\n";
    // This is just a placeholder to satisfy the interface
    return std::unique_ptr<FrontendAction>(action);
}

// Modified from FrontendActionFactory::runInvocation
bool ToolActionWrapper::runInvocation(
        std::shared_ptr<CompilerInvocation> Invocation,
        FileManager *Files,
        std::shared_ptr<PCHContainerOperations> PCHContainerOps,
        DiagnosticConsumer *DiagConsumer) {

    CompilerInstance Compiler(std::move(PCHContainerOps));
    Compiler.setInvocation(std::move(Invocation));
    Compiler.setFileManager(Files);
    
    // Compile with debug info
    CodeGenOptions &CGOpts = Compiler.getCodeGenOpts();
    CGOpts.setDebugInfo(codegenoptions::FullDebugInfo);
    CGOpts.RelocationModel = llvm::Reloc::PIC_;
    CGOpts.OptimizationLevel = 0;

    // Create the compiler's actual diagnostics engine.
    Compiler.createDiagnostics(DiagConsumer, /*ShouldOwnClient=*/false);
    if (!Compiler.hasDiagnostics())
        return false;
    
    Compiler.createSourceManager(*Files);
    
    const bool Success = Compiler.ExecuteAction(*action);
    
    Files->clearStatCache();
    return Success;
}