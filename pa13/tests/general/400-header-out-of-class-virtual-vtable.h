#pragma once

struct HeaderVtableBase
{
  HeaderVtableBase() {}
  virtual int value() = 0;
};

struct HeaderVtableDerived : HeaderVtableBase
{
  HeaderVtableDerived();
  virtual int value();

  int x;
};
