#include "MetricsVisitorConsumer.h"


const std::unordered_set<std::string> MetricsVisitorConsumer::kEmpty{};


MetricsVisitorConsumer::MetricsVisitorConsumer(clang::ASTContext& Ctx)
    : MetricsVisitorConsumer(Ctx, kEmpty) {}

MetricsVisitorConsumer::MetricsVisitorConsumer(
        clang::ASTContext& Ctx,
        const std::unordered_set<std::string>& Covered)
    : Visitor(Ctx, Covered) {}


void MetricsVisitorConsumer::HandleTranslationUnit(clang::ASTContext& Ctx) {
  Visitor.TraverseDecl(Ctx.getTranslationUnitDecl());

  llvm::outs()
      << "pointerDecls: "  << Visitor.pointerDecls  << '\n'
      << "pointerDerefs: " << Visitor.pointerDerefs << '\n'
      << "casts: "         << Visitor.casts         << '\n'
      << "calls: "         << Visitor.calls         << '\n'
      << "unsafeLines: "   << Visitor.unsafeLines   << '\n';
}
