#include <fstream>
#include <unordered_set>
#include <algorithm>
#include <cctype>

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

static inline std::string trim(std::string s) {
  auto notspace = [](int ch){ return !std::isspace(ch); };
  s.erase(s.begin(), std::find_if(s.begin(), s.end(), notspace));
  s.erase(std::find_if(s.rbegin(), s.rend(), notspace).base(), s.end());
  return s;
}

class MetricsFrontendAction : public ASTFrontendAction {
  const std::unordered_set<std::string>& Covered;
  MetricsVisitorConsumer* consumer = nullptr;   // keep pointer to created consumer
  json data;

public:
  explicit MetricsFrontendAction(const std::unordered_set<std::string>& C)
      : Covered(C), data(json::object()) {}

  std::unique_ptr<ASTConsumer>
  CreateASTConsumer(CompilerInstance& CI, StringRef) override {
    auto ptr = std::make_unique<MetricsVisitorConsumer>(CI.getASTContext(), Covered);
    consumer = ptr.get();
    return ptr;
  }

  void EndSourceFileAction() override {
    ASTFrontendAction::EndSourceFileAction();

    if (!consumer) return;
    json sub = consumer->getData();

    // Safely add numeric counters without inserting nulls.
    for (auto &kv : sub.items()) {
      const auto &k = kv.key();
      const auto &v = kv.value();

      if (!v.is_number_integer()) continue;  // ignore non-integer fields defensively
      int rhs = v.get<int>();

      int lhs = 0;
      auto it = data.find(k);
      if (it != data.end() && it->is_number_integer()) {
        lhs = it->get<int>();
      }
      data[k] = lhs + rhs;  // assign AFTER computing lhs (no null insertion math)
    }

    consumer = nullptr; // don’t reuse between TUs
  }

  json getData() const { return data; }
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
    for (std::string line; std::getline(in, line); ) {
      // strip comments and whitespace
      auto hash = line.find('#');
      if (hash != std::string::npos) line.resize(hash);
      line = trim(line);
      if (!line.empty()) Covered.insert(line);
    }
  }

  ClangTool Tool(ExpectedParser->getCompilations(),
                 ExpectedParser->getSourcePathList());

  ToolActionWrapper actionWrapper(new MetricsFrontendAction(Covered));
  Tool.run(&actionWrapper);

  auto *action = dynamic_cast<MetricsFrontendAction*>(actionWrapper.getAction());
  if (!action) {
    errs() << "Failed to cast ToolActionWrapper to MetricsFrontendAction\n";
    return 1;
  }

  json data = action->getData();
  if (data.is_null()) {
    errs() << "No data collected\n";
    return 1;
  }

  for (const auto& item : data.items()) {
    if (!item.value().is_number_integer()) continue;
    outs() << item.key() << ": " << item.value().get<int>() << "\n";
  }

  return 0;
}
