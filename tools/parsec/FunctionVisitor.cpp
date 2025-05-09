#include "FunctionVisitor.h"
#include <clang/AST/Decl.h>

using namespace clang;
using json = nlohmann::json;

bool FunctionVisitor::VisitFunctionDecl(FunctionDecl *function) {
    
    std::string functionName = function->getNameInfo().getName().getAsString();

    if (config.renameMainFunction && functionName == "main") {
        // Here we are assuming that `main` is not called by any other functions
		// If it is, then we need to make this modification while generating the CFG
		// instead of patching it here.
        functionName = "main_0"; // Rename main to main_0
    }
    // Check if functionName is in config.excludedFunctions
    if (std::find(config.excludedFunctions.begin(),
                config.excludedFunctions.end(),
                functionName) != config.excludedFunctions.end()) {
        return true; // Skip this function
    }

    SourceLocation startLocation = function->getBeginLoc();
    SourceManager &SM = context->getSourceManager();
    startLocation = SM.getFileLoc(startLocation); // To compensate for macros
    FullSourceLoc startLoc = context->getFullLoc(startLocation);
    FullSourceLoc endLoc = context->getFullLoc(function->getEndLoc());

    if (!startLoc.isValid() || !endLoc.isValid()) {
        return true;
    }

    if (SM.isInSystemHeader(startLoc)) {
        return true;
    }
    if (function->isThisDeclarationADefinition()) {
        int startLine = startLoc.getSpellingLineNumber();
        int endLine = endLoc.getSpellingLineNumber();
        int startCol = startLoc.getSpellingColumnNumber();
        int endCol = endLoc.getSpellingColumnNumber();

        // Get file name
        std::string fileName = SM.getFilename(startLocation).str();
        // Check if fileName is in config.excludedFiles
        if (std::find(config.excludedFiles.begin(),
                config.excludedFiles.end(),
                fileName) != config.excludedFiles.end()) {
            return true; // Skip this file
        }
        
        PrintingPolicy Policy(context->getLangOpts());
        std::string returnType = function->getReturnType().getAsString(Policy);
        std::vector<std::string> argTypes;
        std::vector<std::string> argNames;
        std::string static_str = function->isStatic() ? "static " : "";
        std::string inline_str = function->isInlineSpecified() ? "inline " : "";
        std::string signature = static_str + inline_str + returnType + " " + function->getQualifiedNameAsString() + "(";

        for (unsigned i = 0; i < function->getNumParams(); ++i) {
            if (i > 0) {
                signature += ", ";
            }
            ParmVarDecl *param = function->getParamDecl(i);
            std::string paramType = param->getType().getAsString(Policy);
            std::string paramName = param->getNameAsString();

            argTypes.push_back(paramType);
            argNames.push_back(paramName);

            signature += paramType;
            if (!paramName.empty()) { // Only add name if it exists
                signature += " " + paramName;
            }
        }
        signature += ")";

        // Add this info to the json data
        json functionData = {
                {"name", functionName},
                {"signature", signature},
                {"num_args", function->getNumParams()},
                {"argTypes", argTypes},
                {"argNames", argNames},
                {"returnType", returnType},
                {"filename", fileName},
                {"startLine", startLine},
                {"endLine", endLine},
                {"startCol", startCol},
                {"endCol", endCol}
        };
        this->data["functions"].push_back(functionData);
    }
    return true;
}



bool FunctionVisitor::VisitRecordDecl(RecordDecl *record) {

    llvm::errs() << "Visiting RecordDecl: " << record->getNameAsString() << "\n";

    if (!record->isStruct() || !record->isThisDeclarationADefinition()) {
        return true;
    }

    SourceManager &SM = context->getSourceManager();
    SourceLocation startLocation = SM.getFileLoc(record->getBeginLoc());
    FullSourceLoc startLoc = context->getFullLoc(startLocation);
    FullSourceLoc endLoc = context->getFullLoc(record->getEndLoc());

    if (!startLoc.isValid() || !endLoc.isValid() || SM.isInSystemHeader(startLoc)) {
        return true;
    }

    std::string fileName = SM.getFilename(startLocation).str();
    if (std::find(config.excludedFiles.begin(), config.excludedFiles.end(), fileName) != config.excludedFiles.end()) {
        return true;
    }

    int startLine = startLoc.getSpellingLineNumber();
    int endLine = endLoc.getSpellingLineNumber();
    int startCol = startLoc.getSpellingColumnNumber();
    int endCol = endLoc.getSpellingColumnNumber();

    json structData = {
        {"name", record->getNameAsString()},
        {"filename", fileName},
        {"startLine", startLine},
        {"endLine", endLine},
        {"startCol", startCol},
        {"endCol", endCol}
    };
    this->data["structures"].push_back(structData);
    return true;
}



bool FunctionVisitor::VisitVarDecl(VarDecl *var) {


    llvm::errs() << "Visiting VarDecl: " << var->getNameAsString() << "\n";

    if (!var->hasGlobalStorage() || !var->isFileVarDecl()) {
        return true;
    }

    SourceManager &SM = context->getSourceManager();
    SourceLocation startLocation = SM.getFileLoc(var->getBeginLoc());
    FullSourceLoc startLoc = context->getFullLoc(startLocation);
    FullSourceLoc endLoc = context->getFullLoc(var->getEndLoc());

    if (!startLoc.isValid() || !endLoc.isValid() || SM.isInSystemHeader(startLoc)) {
        return true;
    }

    std::string fileName = SM.getFilename(startLocation).str();
    if (std::find(config.excludedFiles.begin(), config.excludedFiles.end(), fileName) != config.excludedFiles.end()) {
        return true;
    }

    int startLine = startLoc.getSpellingLineNumber();
    int endLine = endLoc.getSpellingLineNumber();
    int startCol = startLoc.getSpellingColumnNumber();
    int endCol = endLoc.getSpellingColumnNumber();

    PrintingPolicy Policy(context->getLangOpts());

    json globalData = {
        {"name", var->getNameAsString()},
        {"type", var->getType().getAsString(Policy)},
        {"filename", fileName},
        {"startLine", startLine},
        {"endLine", endLine},
        {"startCol", startCol},
        {"endCol", endCol}
    };
    this->data["globals"].push_back(globalData);
    return true;
}
