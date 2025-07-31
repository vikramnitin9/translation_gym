#ifndef METRICSVISITORCONSUMER_H
#define METRICSVISITORCONSUMER_H

#include "MetricsVisitor.h"

#include <unordered_set>
#include "clang/AST/ASTConsumer.h"
#include "clang/Frontend/CompilerInstance.h"
#include "llvm/Support/raw_ostream.h"
#include "nlohmann/json.hpp"

using json = nlohmann::json;

class MetricsVisitorConsumer : public clang::ASTConsumer {
public:

  explicit MetricsVisitorConsumer(clang::ASTContext& Ctx);


  MetricsVisitorConsumer(clang::ASTContext& Ctx,
                         const std::unordered_set<std::string>& Covered);

  void HandleTranslationUnit(clang::ASTContext& Ctx) override;

  json getData() const {
    return data;
  }

private:

  static const std::unordered_set<std::string> kEmpty;
  MetricsVisitor Visitor;
  json data;
};

#endif  // METRICSVISITORCONSUMER_H
