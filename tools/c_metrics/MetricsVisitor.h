#ifndef METRICS_VISITOR_H
#define METRICS_VISITOR_H

#include <unordered_set>
#include <string>

#include "clang/AST/ASTContext.h"
#include "clang/AST/RecursiveASTVisitor.h"
#include "nlohmann/json.hpp"

using json = nlohmann::json;

namespace clang {
  class VarDecl;
  class FieldDecl;
  class UnaryOperator;
  class MemberExpr;
  class ArraySubscriptExpr;
  class CStyleCastExpr;
  class CallExpr;
  class CXXStaticCastExpr;
  class CXXReinterpretCastExpr;
  class CXXConstCastExpr;
  class FunctionDecl;
  class QualType;
}

class MetricsVisitor
  : public clang::RecursiveASTVisitor<MetricsVisitor>
{
public:
  MetricsVisitor(clang::ASTContext &ctx,
                 const std::unordered_set<std::string> &coveredFns);

  /* counting visitors */
  bool VisitVarDecl(clang::VarDecl *D);
  bool VisitFieldDecl(clang::FieldDecl *F);                 // NEW: struct/union fields
  bool VisitUnaryOperator(clang::UnaryOperator *U);
  bool VisitMemberExpr(clang::MemberExpr *M);               // NEW: p->field
  bool VisitArraySubscriptExpr(clang::ArraySubscriptExpr*); // NEW: p[i] with pointer base
  bool VisitCStyleCastExpr(clang::CStyleCastExpr *C);
  bool VisitCXXStaticCastExpr(clang::CXXStaticCastExpr *C);
  bool VisitCXXReinterpretCastExpr(clang::CXXReinterpretCastExpr *C);
  bool VisitCXXConstCastExpr(clang::CXXConstCastExpr *C);
  bool VisitCallExpr(clang::CallExpr *C);

  /* override traversal to detect when we’re inside a covered function */
  bool TraverseFunctionDecl(clang::FunctionDecl *F);

  json getData();

  /* public counters */
  int pointerDecls  = 0;
  int pointerDerefs = 0;
  int casts         = 0;
  int calls         = 0;
  int unsafeLines   = 0;   ///< total physical lines of all covered functions

private:
  // helper: treat typedefs etc. uniformly when checking pointer-ness
  static bool isPointerLike(clang::QualType QT, clang::ASTContext &Ctx);

  clang::ASTContext &context;
  const std::unordered_set<std::string> &covered;
  bool inCovered = false;
};

#endif // METRICS_VISITOR_H
