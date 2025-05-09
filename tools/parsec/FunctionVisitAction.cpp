#include "FunctionVisitAction.h"

using namespace clang;

std::unique_ptr<ASTConsumer>
FunctionVisitAction::CreateASTConsumer(CompilerInstance &compiler, llvm::StringRef inFile) {

    std::unique_ptr<ASTConsumer> defaultConsumer(
        EmitLLVMOnlyAction::CreateASTConsumer(compiler, inFile));

    return std::make_unique<FunctionVisitorConsumer>(
        &compiler.getASTContext(), compiler, std::move(defaultConsumer), this->config);
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
    json subData = myConsumer.getData();
    // Append the updated data to this->data
    for (const auto &entry : subData["functions"]) {
        // Ensure uniqueness
        if (std::find_if(this->data["functions"].begin(), this->data["functions"].end(),
            [&entry](const json &e) { return e["name"] == entry["name"]; }) != this->data["functions"].end()) {
            continue; // Skip if the function already exists
        }
        this->data["functions"].push_back(entry);
    }
    for (const auto &entry : subData["files"]) {
        // Ensure uniqueness
        if (std::find_if(this->data["files"].begin(), this->data["files"].end(),
            [&entry](const json &e) { return e["name"] == entry["name"]; }) != this->data["files"].end()) {
            continue; // Skip if the file already exists
        }
        this->data["files"].push_back(entry);
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