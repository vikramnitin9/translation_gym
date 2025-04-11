#ifndef TOOL_ACTION_WRAPPER_H
#define TOOL_ACTION_WRAPPER_H

#include "clang/Frontend/CompilerInstance.h"
#include "clang/Tooling/Tooling.h"

#include "nlohmann/json.hpp"
#include <unordered_set>
#include <iostream>

using namespace clang;
using json = nlohmann::json;

class ToolActionWrapper: public tooling::FrontendActionFactory {
    public:
        explicit ToolActionWrapper(FrontendAction *action)
            : action(action) {}
        
        FrontendAction *getAction() {
            return action;
        }
    
        // Ideally this function should never be called
        std::unique_ptr<FrontendAction> create() override;
    
        // Modified from FrontendActionFactory::runInvocation
        bool runInvocation(
            std::shared_ptr<CompilerInvocation> Invocation,
            FileManager *Files,
            std::shared_ptr<PCHContainerOperations> PCHContainerOps,
            DiagnosticConsumer *DiagConsumer) override;
    
    private:
        FrontendAction *action;
};

#endif // TOOL_ACTION_WRAPPER_H