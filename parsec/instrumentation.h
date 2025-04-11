#ifndef INSTRUMENTATION_H
#define INSTRUMENTATION_H

#include "llvm/IR/Module.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InstrTypes.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/DerivedTypes.h"

void addInstrumentation(llvm::Module &M);

#endif // INSTRUMENTATION_H