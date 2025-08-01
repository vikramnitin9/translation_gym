#ifndef FUNCTION_VISIT_ACTION_H
#define FUNCTION_VISIT_ACTION_H

#include "clang/Frontend/CompilerInstance.h"
#include "clang/Frontend/FrontendAction.h"
#include "clang/CodeGen/CodeGenAction.h"

#include "llvm/IR/Module.h"
#include "llvm/Linker/Linker.h"

#include "nlohmann/json.hpp"
#include <iostream>

#include "FunctionVisitorConsumer.h"
#include "helpers.h"

using namespace clang;
using json = nlohmann::json;

class FunctionVisitAction : public EmitLLVMOnlyAction {
    public:
        FunctionVisitAction(VisitorConfig config = {false, {}})
            : EmitLLVMOnlyAction(), config(config) {
                
            data = {
                {"files",       json::array()},
                {"functions",   json::array()},
                {"structs",     json::array()},
                {"globals",    json::array()},
                {"enums",      json::array()}
            };
        }
            
        std::unique_ptr<ASTConsumer>
        CreateASTConsumer(CompilerInstance &compiler, llvm::StringRef inFile);
    
        void EndSourceFileAction() override;
    
        json getData() {
            return data;
        }
        std::unique_ptr<llvm::Module> getModule() {
            return std::move(mod);
        }
    
    private:
        VisitorConfig config;
        json data;
        std::unique_ptr<llvm::Module> mod;
        llvm::Linker* linker = nullptr;
};

#endif // FUNCTION_VISIT_ACTION_H