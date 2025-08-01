#ifndef FUNCTION_VISITOR_H
#define FUNCTION_VISITOR_H

#include "clang/AST/ASTConsumer.h"
#include "clang/AST/RecursiveASTVisitor.h"
#include "clang/Frontend/CompilerInstance.h"
#include "clang/AST/PrettyPrinter.h"
#include "clang/Lex/Lexer.h"

#include "nlohmann/json.hpp"
#include <unordered_set>
#include <iostream>
#include "helpers.h"

using namespace clang;
using json = nlohmann::json;

class FunctionVisitor : public RecursiveASTVisitor<FunctionVisitor> {
    public:
        explicit FunctionVisitor(ASTContext *_context, CompilerInstance &_compiler, VisitorConfig config = {false, {}})
            : context(_context), compiler(_compiler), config(config) {

            data = {
                {"files", json::array()},
                {"functions", json::array()},
                {"structs", json::array()},
                {"globals", json::array()},
                {"enums", json::array()}
            };
        }

        bool VisitFunctionDecl(FunctionDecl *function);
        bool VisitTagDecl(TagDecl *tag);
        bool VisitVarDecl(VarDecl *var);
        bool VisitDeclRefExpr(DeclRefExpr *expr);
        bool VisitTypeLoc(TypeLoc typeLoc);

        json getData() {
            return data;
        }

    private:
        VisitorConfig config;
        ASTContext *context;
        CompilerInstance &compiler;
        std::unordered_set<std::string> currentGlobals;
        std::unordered_set<std::string> currentStructs;
        std::unordered_set<std::string> currentEnums;
        json data;
};
#endif // FUNCTION_VISITOR_H