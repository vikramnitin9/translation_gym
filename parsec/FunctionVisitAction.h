#ifndef FUNCTION_VISIT_ACTION_H
#define FUNCTION_VISIT_ACTION_H

#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/FrontendAction.h"
#include "clang/CodeGen/CodeGenAction.h"

#include "llvm/IR/Module.h"
#include "llvm/Linker/Linker.h"

#include "nlohmann/json.hpp"
#include <unordered_set>
#include <iostream>

#include "FunctionVisitorConsumer.h"

using namespace clang;
using json = nlohmann::json;

class FunctionVisitAction : public EmitLLVMOnlyAction {
    public:
        std::unique_ptr<ASTConsumer>
        CreateASTConsumer(CompilerInstance &compiler, llvm::StringRef inFile);
    
        void EndSourceFileAction() override;
    
        std::unordered_set<json> getData() {
            return data;
        }
        std::unique_ptr<llvm::Module> getModule() {
            return std::move(mod);
        }
    
    private:
        std::unordered_set<json> data;
        std::unique_ptr<llvm::Module> mod;
        llvm::Linker* linker = nullptr;
};

#endif // FUNCTION_VISIT_ACTION_H