#include "MetricsVisitor.h"

#include "clang/AST/Decl.h"
#include "clang/AST/Expr.h"
#include "clang/AST/Type.h"
#include "clang/Basic/SourceManager.h"

using namespace clang;


MetricsVisitor::MetricsVisitor(ASTContext &ctx,
                               const std::unordered_set<std::string> &coveredFns)
    : context(ctx), covered(coveredFns) {}


bool MetricsVisitor::TraverseFunctionDecl(FunctionDecl *F) {
  bool oldCovered = inCovered;

  const std::string name = F ? F->getNameAsString() : "";
  bool countMe = covered.empty() || covered.count(name);

  if (F && F->hasBody() && countMe) {
    inCovered = true;

    const SourceManager &SM = context.getSourceManager();
    SourceLocation begin = F->getBody()->getBeginLoc();
    SourceLocation end   = F->getBody()->getEndLoc();
    unsigned l1 = SM.getSpellingLineNumber(begin);
    unsigned l2 = SM.getSpellingLineNumber(end);
    unsafeLines += (l2 >= l1) ? (l2 - l1 + 1) : 0;
  } else {
    inCovered = false;
  }

  RecursiveASTVisitor::TraverseFunctionDecl(F);
  inCovered = oldCovered;
  return true;
}

//counting visitors (apply only when inCovered)
bool MetricsVisitor::VisitVarDecl(VarDecl *D) {
  if (inCovered && D->getType()->isPointerType())
    ++pointerDecls;
  return true;
}

bool MetricsVisitor::VisitUnaryOperator(UnaryOperator *U) {
  if (inCovered && U->getOpcode() == UO_Deref)
    ++pointerDerefs;
  return true;
}

bool MetricsVisitor::VisitCStyleCastExpr(CStyleCastExpr *) {
  if (inCovered) ++casts;
  return true;
}

bool MetricsVisitor::VisitCXXStaticCastExpr(CXXStaticCastExpr *) {
  if (inCovered) ++casts;
  return true;
}

bool MetricsVisitor::VisitCXXReinterpretCastExpr(CXXReinterpretCastExpr *) {
  if (inCovered) ++casts;
  return true;
}

bool MetricsVisitor::VisitCXXConstCastExpr(CXXConstCastExpr *) {
  if (inCovered) ++casts;
  return true;
}

bool MetricsVisitor::VisitCallExpr(CallExpr *) {
  if (inCovered) ++calls;
  return true;
}

json MetricsVisitor::getData() {
    json data;
    data["pointerDecls"]  = pointerDecls;
    data["pointerDerefs"] = pointerDerefs;
    data["casts"]         = casts;
    data["calls"]         = calls;
    data["unsafeLines"]   = unsafeLines;
    return data;
}