namespace lib {
inline namespace abi {
template<class Key, class T, class Ignored, class Alloc>
struct ordered_map { int value; };

template<template<class U, class V, class... Args> class ObjectType>
class basic_json;

using ordered_json = basic_json<lib::ordered_map>;

template<template<class, class, class...> class ObjectType>
class basic_json {
public:
  typedef ObjectType<int, basic_json, char, short> object_t;
};
}
}

int main() {
  lib::ordered_json::object_t object;
  object.value = 7;
  return object.value != 7;
}
