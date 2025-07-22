// tools/C_metrics/MetricsVisitorConsumer.h
#ifndef METRICSVISITORCONSUMER_H
#define METRICSVISITORCONSUMER_H

#include "clang/AST/ASTConsumer.h"
#include "clang/Frontend/CompilerInstance.h"
#include "MetricsVisitor.h"
#include "llvm/Support/raw_ostream.h"

class MetricsVisitorConsumer : public clang::ASTConsumer {
public:
  explicit MetricsVisitorConsumer(clang::ASTContext &Ctx)
    : Visitor(Ctx)
  {}

  void HandleTranslationUnit(clang::ASTContext &Ctx) override {
    Visitor.TraverseDecl(Ctx.getTranslationUnitDecl());
    llvm::outs()
      << "pointerDecls: "   << Visitor.pointerDecls   << "\n"
      << "pointerDerefs: "  << Visitor.pointerDerefs  << "\n"
      << "casts: "          << Visitor.casts          << "\n"
      << "calls: "          << Visitor.calls          << "\n";
  }

private:
  MetricsVisitor Visitor;
};

#endif // METRICSVISITORCONSUMER_H
