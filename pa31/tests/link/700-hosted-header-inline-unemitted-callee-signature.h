#pragma once

#include <streambuf>

struct HeaderInlineBase
{
  HeaderInlineBase() {}
  virtual int next() = 0;
};

struct HeaderInlineDerived : HeaderInlineBase
{
  explicit HeaderInlineDerived(std::streambuf * buf) : buf(buf) {}
  virtual int next();

  std::streambuf * buf;
};
