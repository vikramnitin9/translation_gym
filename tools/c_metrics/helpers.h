#ifndef HELPERS_H
#define HELPERS_H

#include <string>
#include <vector>

struct VisitorConfig {
    bool renameMainFunction;
    std::vector<std::string> excludedFunctions;
    std::vector<std::string> excludedFiles;
};

#endif // HELPERS_H