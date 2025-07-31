#include <fstream>
#include <unordered_set>

#include "MetricsVisitorConsumer.h"
#include "ToolActionWrapper.h"

#include "clang/Tooling/CommonOptionsParser.h"
#include "clang/Tooling/Tooling.h"
#include "clang/Frontend/FrontendActions.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/raw_ostream.h"
#include "nlohmann/json.hpp"

using namespace clang;
using namespace clang::tooling;
using namespace llvm;
using json = nlohmann::json;

static cl::OptionCategory MetricsCategory("c-metrics options");

static cl::opt<std::string> CoveredPath(
  "covered-functions",
  cl::desc("TXT file with one function name per line"),
  cl::value_desc("path"),
  cl::init(""),
  cl::cat(MetricsCategory));


class MetricsFrontendAction : public ASTFrontendAction {
  const std::unordered_set<std::string>& Covered;
  json data;
public:
  explicit MetricsFrontendAction(const std::unordered_set<std::string>& C)
      : Covered(C) {
    data = json::object();
  }

  std::unique_ptr<ASTConsumer>
  CreateASTConsumer(CompilerInstance& CI, StringRef) override {
    return std::make_unique<MetricsVisitorConsumer>(CI.getASTContext(),
                                                    Covered);
  }

  void EndSourceFileAction() override {
    // First call the base class implementation
    ASTFrontendAction::EndSourceFileAction();
    // Next, retrieve the ASTConsumer so that we can get the stored data
    CompilerInstance &compiler = this->getCompilerInstance();
    if (!compiler.hasASTConsumer()) {
      std::cerr << "No ASTConsumer\n";
      return;
    }
    MetricsVisitorConsumer &myConsumer = dynamic_cast<MetricsVisitorConsumer&>(compiler.getASTConsumer());
    json subData = myConsumer.getData();
    // Add the updated counts to this->data
    for (auto& item : subData.items()) {
      if (item.value().is_null()) {
        continue; // Skip null values
      }
      const auto& key = item.key();
      const auto& value = item.value();
      this->data[key] = this->data.value(key, 0) + value.get<int>();
    }
  }

  json getData() const {
    return data;
  }
};

int main(int argc, const char** argv) {
  auto ExpectedParser = CommonOptionsParser::create(
      argc, argv, MetricsCategory, llvm::cl::OneOrMore,
      "c-metrics (counts pointers, casts, calls)");

  if (!ExpectedParser) {
    errs() << toString(ExpectedParser.takeError()) << '\n';
    return 1;
  }

  std::unordered_set<std::string> Covered;
  if (!CoveredPath.empty()) {
    std::ifstream in(CoveredPath);
    for (std::string line; std::getline(in, line); )
      if (!line.empty()) Covered.insert(line);
  }

  ClangTool Tool(ExpectedParser->getCompilations(),
                 ExpectedParser->getSourcePathList());

  ToolActionWrapper actionWrapper(new MetricsFrontendAction(Covered));
  Tool.run(&actionWrapper);
  MetricsFrontendAction* action = dynamic_cast<MetricsFrontendAction*>(actionWrapper.getAction());
  if (!action) {
    errs() << "Failed to cast ToolActionWrapper to MetricsFrontendAction\n";
    return 1;
  }
  json data = action->getData();
  // Print the JSON data
  if (data.is_null()) {
    errs() << "No data collected\n";
    return 1;
  }
  for (const auto& item : data.items()) {
    if (item.value().is_null()) {
      continue; // Skip null values
    }
    llvm::outs() << item.key() << ": " << item.value().get<int>() << "\n";
  }

  return 0;
}
