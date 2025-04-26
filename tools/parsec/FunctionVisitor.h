#ifndef FUNCTION_VISITOR_H
#define FUNCTION_VISITOR_H

#include "clang/AST/ASTConsumer.h"
#include "clang/AST/RecursiveASTVisitor.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/AST/PrettyPrinter.h"

#include "nlohmann/json.hpp"
#include <unordered_set>

using namespace clang;
using json = nlohmann::json;

class FunctionVisitor : public RecursiveASTVisitor<FunctionVisitor> {
    public:
        explicit FunctionVisitor(ASTContext *_context, CompilerInstance &_compiler)
        : context(_context), compiler(_compiler) {}

        bool VisitFunctionDecl(FunctionDecl *function);

        std::unordered_set<json> getData() {
            return data;
        }

    private:
        ASTContext *context;
        CompilerInstance &compiler;
        std::unordered_set<json> data;
};
#endif // FUNCTION_VISITOR_H