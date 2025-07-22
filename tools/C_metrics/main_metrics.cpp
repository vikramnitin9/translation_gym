// tools/C_metrics/main_metrics.cpp
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

class MetricsFrontendAction : public ASTFrontendAction {
public:
  std::unique_ptr<ASTConsumer>
  CreateASTConsumer(CompilerInstance &CI, StringRef) override {
    return std::make_unique<MetricsVisitorConsumer>(CI.getASTContext());
  }
};

int main(int argc, const char **argv) {
  auto ExpectedParser = CommonOptionsParser::create(
    argc, argv,
    MetricsCategory,
    cl::Required,                          // <— add this
    "c-metrics (counts pointers, casts, calls)"  // <— and this
  );
  if (!ExpectedParser) {
    errs() << toString(ExpectedParser.takeError()) << "\n";
    return 1;
  }

  ClangTool Tool(
    ExpectedParser->getCompilations(),
    ExpectedParser->getSourcePathList()
  );

  return Tool.run(newFrontendActionFactory<MetricsFrontendAction>().get());
}
