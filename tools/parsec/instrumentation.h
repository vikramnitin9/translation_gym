#ifndef INSTRUMENTATION_H
#define INSTRUMENTATION_H

#include "llvm/IR/Module.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InstrTypes.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/DebugInfoMetadata.h"
#include "llvm/Support/Path.h"

#include "nlohmann/json.hpp"
using json = nlohmann::json;

bool compareFilenames(std::string filename1, std::string filename2);
void addInstrumentation(llvm::Module &M, json jsonData);

extern const char* cpp_source;

#endif // INSTRUMENTATION_H