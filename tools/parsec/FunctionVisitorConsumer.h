#ifndef FUNCTIONVISITORCONSUMER_H
#define FUNCTIONVISITORCONSUMER_H

#include "clang/AST/ASTConsumer.h"
#include "clang/Frontend/CompilerInstance.h"

#include "nlohmann/json.hpp"
#include <unordered_set>

#include "FunctionVisitor.h"
#include "helpers.h"

using namespace clang;
using json = nlohmann::json;

class FunctionVisitorConsumer : public ASTConsumer {
    public:
        explicit FunctionVisitorConsumer(ASTContext *context,
                                         CompilerInstance &compiler,
                                         std::unique_ptr<ASTConsumer> defaultConsumer,
                                         VisitorConfig config = {false, {}})
                : visitor(context, compiler, config), defaultConsumer(std::move(defaultConsumer)) {}
    
        void HandleTranslationUnit(ASTContext &context);

        json getData() {
            return data;
        }
    
        // Here we forward the calls to the default consumer
        // Ugly, but I don't know a better way
        void Initialize(ASTContext &Context) { defaultConsumer->Initialize(Context); }
        bool HandleTopLevelDecl(DeclGroupRef D) { return defaultConsumer->HandleTopLevelDecl(D); }
        void HandleInlineFunctionDefinition(FunctionDecl *D) { defaultConsumer->HandleInlineFunctionDefinition(D); }
        void HandleInterestingDecl(DeclGroupRef D) { defaultConsumer->HandleInterestingDecl(D); }
        void HandleTagDeclDefinition(TagDecl *D) { defaultConsumer->HandleTagDeclDefinition(D); }
        void HandleTagDeclRequiredDefinition(const TagDecl *D) { defaultConsumer->HandleTagDeclRequiredDefinition(D); }
        void HandleCXXImplicitFunctionInstantiation(FunctionDecl *D) { defaultConsumer->HandleCXXImplicitFunctionInstantiation(D); }
        void HandleTopLevelDeclInObjCContainer(DeclGroupRef D) { defaultConsumer->HandleTopLevelDeclInObjCContainer(D); }
        void HandleImplicitImportDecl(ImportDecl *D) { defaultConsumer->HandleImplicitImportDecl(D); }
        void CompleteTentativeDefinition(VarDecl *D) { defaultConsumer->CompleteTentativeDefinition(D); }
        void CompleteExternalDeclaration(VarDecl *D) { defaultConsumer->CompleteExternalDeclaration(D); }
        void AssignInheritanceModel(CXXRecordDecl *RD) { defaultConsumer->AssignInheritanceModel(RD); }
        void HandleCXXStaticMemberVarInstantiation(VarDecl *D) { defaultConsumer->HandleCXXStaticMemberVarInstantiation(D); }
        void HandleVTable(CXXRecordDecl *RD) { defaultConsumer->HandleVTable(RD); }
        ASTMutationListener *GetASTMutationListener() { return defaultConsumer->GetASTMutationListener(); }
        ASTDeserializationListener *GetASTDeserializationListener() { return defaultConsumer->GetASTDeserializationListener(); }
        void PrintStats() { defaultConsumer->PrintStats(); }
        bool shouldSkipFunctionBody(Decl *D) { return defaultConsumer->shouldSkipFunctionBody(D); }
    
    private:
        FunctionVisitor visitor;
        std::unique_ptr<ASTConsumer> defaultConsumer;
        json data;
};

#endif // FUNCTIONVISITORCONSUMER_H