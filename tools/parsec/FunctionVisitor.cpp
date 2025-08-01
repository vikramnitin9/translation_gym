#include "FunctionVisitor.h"


using namespace clang;
using json = nlohmann::json;

bool FunctionVisitor::VisitFunctionDecl(FunctionDecl *function) {

    std::string functionName = function->getNameInfo().getName().getAsString();

     // only definitions have bodies
    if (!function->hasBody() || !function->isThisDeclarationADefinition()) {
        // For main function alone, we retain it even if it has no body
        if (functionName != "main") {
            return true;
        }
    }
    // Clear last-function’s deps
    currentGlobals.clear();
    currentStructs.clear();
    currentEnums.clear();


    // Traverse the function body to collect calls, globals, structs
    RecursiveASTVisitor<FunctionVisitor>::TraverseStmt(function->getBody());

    // Print currentStructs, currentEnums, and currentGlobals
    for (const auto &s : currentStructs) {
    }
    for (const auto &e : currentEnums) {
    }
    for (const auto &g : currentGlobals) {
    }
    
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
    FullSourceLoc endLoc = context->getFullLoc(SM.getFileLoc(function->getEndLoc()));

    if (!startLoc.isValid() || !endLoc.isValid()) {
        return true;
    }

    if (SM.isInSystemHeader(startLoc)) {
        return true;
    }
    if (function->isThisDeclarationADefinition() || functionName == "main_0") {
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
                {"name",        functionName},
                {"signature",   signature},
                {"num_args",    function->getNumParams()},
                {"argTypes",    argTypes},
                {"argNames",    argNames},
                {"returnType",  returnType},
                {"filename",    fileName},
                {"startLine",   startLine},
                {"endLine",     endLine},
                {"startCol",    startCol},
                {"endCol",      endCol},
                {"functions",   json::array()}
        };
    
        json global_arr = json::array();
        for (auto &n : currentGlobals) {
            json obj;
            obj["name"] = n;
            global_arr.push_back(std::move(obj));
        }
        functionData["globals"] = std::move(global_arr);

        json struct_arr = json::array();
        for (auto &n : currentStructs) {
            json obj;
            obj["name"] = n;
            struct_arr.push_back(std::move(obj));
        }
        functionData["structs"] = std::move(struct_arr);

        json enum_arr = json::array();
        for (auto &n : currentEnums) {
            json obj;
            obj["name"] = n;
            enum_arr.push_back(std::move(obj));
        }
        functionData["enums"] = std::move(enum_arr);

        this->data["functions"].push_back(functionData);
    }
    return true;
}

bool FunctionVisitor::VisitTagDecl(TagDecl *tag) {

    if (!(tag->isStruct() || tag->isEnum()) || !tag->isThisDeclarationADefinition()) {
        return true;
    }
    
    // Struct declarations can depend on other structs and enums
    if (tag->isStruct()) {
        currentStructs.clear();
        currentEnums.clear();
        // Traverse this declaration's fields to collect referenced enums and structs
        if (const auto *RD = dyn_cast<RecordDecl>(tag)) {
            for (FieldDecl *FD : RD->fields()) {
                FunctionVisitor::TraverseDecl(FD); // This triggers VisitTypeLoc()
            }
        }
    }

    SourceManager &SM = context->getSourceManager();
    SourceLocation startLocation = SM.getFileLoc(tag->getBeginLoc());
    FullSourceLoc startLoc = context->getFullLoc(startLocation);
    FullSourceLoc endLoc = context->getFullLoc(SM.getFileLoc(tag->getEndLoc()));

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

    json recordData = {
        {"name", tag->getNameAsString()},
        {"filename", fileName},
        {"startLine", startLine},
        {"endLine", endLine},
        {"startCol", startCol},
        {"endCol", endCol}
    };
    if (tag->isStruct()) {
        json struct_arr = json::array();
        for (auto &n : currentStructs) {
            json obj;
            obj["name"] = n;
            struct_arr.push_back(std::move(obj));
        }
        recordData["structs"] = std::move(struct_arr);
        json enum_arr = json::array();
        for (auto &n : currentEnums) {
            json obj;
            obj["name"] = n;
            enum_arr.push_back(std::move(obj));
        }
        recordData["enums"] = std::move(enum_arr);
    }
    if (tag->isStruct())
        this->data["structs"].push_back(recordData);
    else if (tag->isEnum())
        this->data["enums"].push_back(recordData);
    return true;
}

bool FunctionVisitor::VisitVarDecl(VarDecl *var) {

    if (!var->hasGlobalStorage() || !var->isFileVarDecl() || !var->isThisDeclarationADefinition()) {
        return true;
    }

    SourceManager &SM = context->getSourceManager();
    SourceLocation startLocation = var->getBeginLoc();
    SourceLocation endLocation = var->getEndLoc();
    // If there is a declaration without assignment, like `int abcd`, then
    // the end location is just the first character of the variable name, for some reason
    // So it stops at `int a`. We need to get the end of the token
    endLocation = Lexer::getLocForEndOfToken(endLocation, 0, SM, context->getLangOpts());
    FullSourceLoc startLoc = context->getFullLoc(SM.getFileLoc(startLocation));
    FullSourceLoc endLoc = context->getFullLoc(SM.getFileLoc(endLocation));

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
        {"endCol", endCol},
        {"isStatic", var->getStorageClass() == SC_Static}
    };
    this->data["globals"].push_back(globalData);
    return true;
}

// Record every referenced global VarDecl
bool FunctionVisitor::VisitDeclRefExpr(DeclRefExpr *expr) {
    if (auto *vd = llvm::dyn_cast<VarDecl>(expr->getDecl())) {
        if (vd->hasGlobalStorage() && vd->isFileVarDecl()) {
            currentGlobals.insert(vd->getNameAsString());
        }
    }
    else if (auto *ed = llvm::dyn_cast<EnumConstantDecl>(expr->getDecl())) {
        if (auto *Enum = dyn_cast<EnumDecl>(ed->getDeclContext())) {
            if (Enum->getIdentifier())
                currentEnums.insert(Enum->getName());
        }
    }
    return true;
}

// Record every struct name mentioned in a type
bool FunctionVisitor::VisitTypeLoc(TypeLoc typeLoc) {
    if (auto *tt = typeLoc.getType()->getAs<TagType>()) {
        TagDecl *td = tt->getDecl();
        if (td->isStruct() && td->isThisDeclarationADefinition())
            currentStructs.insert(td->getNameAsString());
        else if (td->isEnum() && td->isThisDeclarationADefinition())
            currentEnums.insert(td->getNameAsString());
    }
    return true;
}
