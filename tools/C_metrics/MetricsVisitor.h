// tools/C_metrics/MetricsVisitor.h
#ifndef METRICS_VISITOR_H
#define METRICS_VISITOR_H

#include "clang/AST/ASTContext.h"
#include "clang/AST/RecursiveASTVisitor.h"

namespace clang {
  class VarDecl;
  class UnaryOperator;
  class CStyleCastExpr;
  class CallExpr;
  class CXXStaticCastExpr;
  class CXXReinterpretCastExpr;
  class CXXConstCastExpr;
}

class MetricsVisitor
  : public clang::RecursiveASTVisitor<MetricsVisitor>
{
public:
  explicit MetricsVisitor(clang::ASTContext &ctx)
    : context(ctx)
    , pointerDecls(0)
    , pointerDerefs(0)
    , casts(0)
    , calls(0)
  {}

  bool VisitVarDecl(clang::VarDecl *D);
  bool VisitUnaryOperator(clang::UnaryOperator *U);
  bool VisitCStyleCastExpr(clang::CStyleCastExpr *C);
  bool VisitCXXStaticCastExpr(clang::CXXStaticCastExpr *C);
  bool VisitCXXReinterpretCastExpr(clang::CXXReinterpretCastExpr *C);
  bool VisitCXXConstCastExpr(clang::CXXConstCastExpr *C);
  bool VisitCallExpr(clang::CallExpr *C);

  int pointerDecls;
  int pointerDerefs;
  int casts;
  int calls;

private:
  clang::ASTContext &context;
};

#endif // METRICS_VISITOR_H
