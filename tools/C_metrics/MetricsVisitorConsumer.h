#ifndef METRICSVISITORCONSUMER_H
#define METRICSVISITORCONSUMER_H

#include <unordered_set>
#include "clang/AST/ASTConsumer.h"
#include "clang/Frontend/CompilerInstance.h"
#include "MetricsVisitor.h"
#include "llvm/Support/raw_ostream.h"

class MetricsVisitorConsumer : public clang::ASTConsumer {
public:

  explicit MetricsVisitorConsumer(clang::ASTContext& Ctx);


  MetricsVisitorConsumer(clang::ASTContext& Ctx,
                         const std::unordered_set<std::string>& Covered);

  void HandleTranslationUnit(clang::ASTContext& Ctx) override;

private:

  static const std::unordered_set<std::string> kEmpty;
  MetricsVisitor Visitor;
};

#endif  // METRICSVISITORCONSUMER_H
