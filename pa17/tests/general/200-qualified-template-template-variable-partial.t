// VALIDATION: compile-pass

namespace lib {

template<class T, template<class...> class Template>
inline const bool is_specialization_v = false;

template<template<class...> class Template, class... Args>
inline const bool is_specialization_v<Template<Args...>, Template> = true;

template<class First, class Second>
struct pair
{
};

}

static_assert(lib::is_specialization_v<lib::pair<int, int>, lib::pair>,
              "qualified variable-template partial specialization");

int main()
{
  return 0;
}
