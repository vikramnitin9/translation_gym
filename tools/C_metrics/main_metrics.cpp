#include <fstream>
#include <unordered_set>

#include "MetricsVisitorConsumer.h"
#include "clang/Tooling/CommonOptionsParser.h"
#include "clang/Tooling/Tooling.h"
#include "clang/Frontend/FrontendActions.h"
#include "llvm/Support/CommandLine.h"
#include "llvm/Support/raw_ostream.h"

using namespace clang;
using namespace clang::tooling;
using namespace llvm;

static cl::OptionCategory MetricsCategory("c-metrics options");

static cl::opt<std::string> CoveredPath(
    "covered-functions",
    cl::desc("TXT file with one function name per line"),
    cl::value_desc("path"),
    cl::init(""),
    cl::cat(MetricsCategory));


class MetricsFrontendAction : public ASTFrontendAction {
  const std::unordered_set<std::string>& Covered;
public:
  explicit MetricsFrontendAction(const std::unordered_set<std::string>& C)
      : Covered(C) {}

  std::unique_ptr<ASTConsumer>
  CreateASTConsumer(CompilerInstance& CI, StringRef) override {
    return std::make_unique<MetricsVisitorConsumer>(CI.getASTContext(),
                                                    Covered);
  }
};


class CoveredFactory : public FrontendActionFactory {
  const std::unordered_set<std::string>& CoveredRef;
public:
  explicit CoveredFactory(const std::unordered_set<std::string>& C)
      : CoveredRef(C) {}

  std::unique_ptr<FrontendAction> create() override {
    return std::make_unique<MetricsFrontendAction>(CoveredRef);
  }
};


int main(int argc, const char** argv) {
  auto ExpectedParser = CommonOptionsParser::create(
      argc, argv, MetricsCategory, cl::Required,
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

  auto Factory = std::make_unique<CoveredFactory>(Covered);
  return Tool.run(Factory.get());
}
