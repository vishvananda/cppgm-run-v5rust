// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param]

namespace ttp_pack {

template<class... T>
struct list
{
};

template<template<class...> class... F>
struct compose
{
};

int test()
{
  return 0;
}

}  // namespace ttp_pack

int main()
{
  return ttp_pack::test();
}
