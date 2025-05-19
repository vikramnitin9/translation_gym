#include "FunctionVisitor.h"

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
        const LangOptions langOpts = context->getLangOpts();
        PrintingPolicy Policy(langOpts);
        std::string returnType = function->getReturnType().getAsString(Policy);
        std::vector<std::string> argTypes;
        std::vector<std::string> argNames;

        for (unsigned i = 0; i < function->getNumParams(); ++i) {
            ParmVarDecl *param = function->getParamDecl(i);
            std::string paramType = param->getType().getAsString(Policy);
            std::string paramName = param->getNameAsString();
            argTypes.push_back(paramType);
            argNames.push_back(paramName);
        }

        SourceLocation loc = startLocation;
        std::string signature;

        // Keep iterating until we reach '{' or run out of tokens
        while (true) {
            // Get the token starting at this location
            Token token;
            bool invalid = Lexer::getRawToken(loc, token, SM, langOpts, true);
            if (invalid)
                break;

            if (token.is(tok::l_brace)) // found '{'
                break;

            // Append token spelling to the signature
            signature += Lexer::getSpelling(token, SM, langOpts) + " ";

            // Advance to next token
            loc = token.getEndLoc();

            // Avoid infinite loop if we can't advance
            if (loc == token.getLocation())
                break;
        }

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