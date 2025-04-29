#include "FunctionVisitAction.h"

using namespace clang;

std::unique_ptr<ASTConsumer>
FunctionVisitAction::CreateASTConsumer(CompilerInstance &compiler, llvm::StringRef inFile) {

    std::unique_ptr<ASTConsumer> defaultConsumer(
        EmitLLVMOnlyAction::CreateASTConsumer(compiler, inFile));

    return std::make_unique<FunctionVisitorConsumer>(
        &compiler.getASTContext(), compiler, std::move(defaultConsumer));
}

void FunctionVisitAction::EndSourceFileAction() {
    // First call the base class implementation
    EmitLLVMOnlyAction::EndSourceFileAction();

    // Next, retrieve the ASTConsumer so that we can get the stored data
    CompilerInstance &compiler = this->getCompilerInstance();
    if (!compiler.hasASTConsumer()) {
        std::cerr << "No ASTConsumer\n";
        return;
    }
    FunctionVisitorConsumer &myConsumer = dynamic_cast<FunctionVisitorConsumer&>(compiler.getASTConsumer());
    std::unordered_set<json> fileData = myConsumer.getData();
    // Append the updated data to this->data
    for (const auto &entry : fileData) {
        // Get "filename" from the entry json object
        std::string filename = entry["filename"];
        this->data.insert(entry);
    }
    // Get the generated module and add it to the list
    std::unique_ptr<llvm::Module> M = this->takeModule();
    if (!M) {
        std::cerr << "Failed to generate module\n";
        return;
    }
    if (linker == nullptr) {
        this->mod = std::move(M);
        linker = new llvm::Linker(*(this->mod));
    } else {
        if (linker->linkInModule(std::move(M))) {
            std::cerr << "Error linking module\n";
            return;
        }
    }
}