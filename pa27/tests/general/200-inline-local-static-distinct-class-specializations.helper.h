#pragma once

template<class T> struct slot
{
  static int * get() { static int value; return &value; }
};

template<class T> struct map {};
