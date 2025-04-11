#include "FunctionVisitorConsumer.h"

using namespace clang;

void FunctionVisitorConsumer::HandleTranslationUnit(ASTContext &context) {
    visitor.TraverseDecl(context.getTranslationUnitDecl());
    data = visitor.getData();
    defaultConsumer->HandleTranslationUnit(context);
}