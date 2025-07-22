// tools/C_metrics/MetricsVisitor.cpp
#include "MetricsVisitor.h"
#include "clang/AST/Decl.h"
#include "clang/AST/Expr.h"
#include "clang/AST/Type.h"

using namespace clang;

bool MetricsVisitor::VisitVarDecl(VarDecl *D) {
  if (D->getType()->isPointerType())
    ++pointerDecls;
  return true;  
}

bool MetricsVisitor::VisitUnaryOperator(UnaryOperator *U) {
  if (U->getOpcode() == UO_Deref)
    ++pointerDerefs;
  return true;
}

bool MetricsVisitor::VisitCStyleCastExpr(CStyleCastExpr *C) {
  ++casts;
  return true;
}

bool MetricsVisitor::VisitCXXStaticCastExpr(CXXStaticCastExpr *C) {
  ++casts;
  return true;
}

bool MetricsVisitor::VisitCXXReinterpretCastExpr(CXXReinterpretCastExpr *C) {
  ++casts;
  return true;
}

bool MetricsVisitor::VisitCXXConstCastExpr(CXXConstCastExpr *C) {
  ++casts;
  return true;
}

bool MetricsVisitor::VisitCallExpr(CallExpr *C) {
  ++calls;
  return true;
}
