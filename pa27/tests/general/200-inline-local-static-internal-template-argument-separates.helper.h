#pragma once

template<class T> int * slot()
{
  static int value;
  return &value;
}

namespace { struct tag {}; }
