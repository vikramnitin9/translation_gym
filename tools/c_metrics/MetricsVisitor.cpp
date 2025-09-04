#include "MetricsVisitor.h"

#include "clang/AST/Decl.h"
#include "clang/AST/Expr.h"
#include "clang/AST/Type.h"
#include "clang/Basic/SourceManager.h"

using namespace clang;

/* ==== helper: canonicalize type before pointer test (handles typedefs) ==== */
bool MetricsVisitor::isPointerLike(QualType QT, ASTContext &Ctx) {
  if (QT.isNull()) return false;
  QualType Canon = QT.getCanonicalType(); // desugar typedefs/aliases
  return Canon->isPointerType();
}

/* ==== ctor ==== */
MetricsVisitor::MetricsVisitor(ASTContext &ctx,
                               const std::unordered_set<std::string> &coveredFns)
    : context(ctx), covered(coveredFns) {}

/* ==== coverage-scoped traversal (unsafeLines unchanged) ==== */
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

/* ==== pointer declaration counting ==== */

// VarDecls: count if inside covered fn OR if they have global/file storage.
bool MetricsVisitor::VisitVarDecl(VarDecl *D) {
  if (!D) return true;

  bool isFileScope = D->isFileVarDecl();    // TU-scope var
  bool isGlobal    = D->hasGlobalStorage(); // includes static at file scope

  if ((inCovered || isFileScope || isGlobal) &&
      isPointerLike(D->getType(), context)) {
    ++pointerDecls;
  }
  return true;
}

// FieldDecls (struct/union members): always count if pointer-typed.
bool MetricsVisitor::VisitFieldDecl(FieldDecl *F) {
  if (F && isPointerLike(F->getType(), context))
    ++pointerDecls;
  return true;
}

/* ==== pointer dereference counting ==== */

// *p
bool MetricsVisitor::VisitUnaryOperator(UnaryOperator *U) {
  if (inCovered && U && U->getOpcode() == UO_Deref)
    ++pointerDerefs;
  return true;
}

// p->field
bool MetricsVisitor::VisitMemberExpr(MemberExpr *M) {
  if (inCovered && M && M->isArrow())
    ++pointerDerefs;
  return true;
}

// p[i]  (when base is a pointer, not a real array variable)
bool MetricsVisitor::VisitArraySubscriptExpr(ArraySubscriptExpr *A) {
  if (!inCovered || !A) return true;

  Expr *Base = A->getBase();
  if (!Base) return true;

  QualType T = Base->IgnoreParenImpCasts()->getType();
  if (isPointerLike(T, context))
    ++pointerDerefs;

  return true;
}

/* ==== casts & calls (unchanged) ==== */
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

/* ==== summary ==== */
json MetricsVisitor::getData() {
    json data;
    data["pointerDecls"]  = pointerDecls;
    data["pointerDerefs"] = pointerDerefs;
    data["casts"]         = casts;
    data["calls"]         = calls;
    data["unsafeLines"]   = unsafeLines;
    return data;
}
