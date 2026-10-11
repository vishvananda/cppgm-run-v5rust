#include <cstddef>
#include <typeinfo>

typedef decltype(typeid(int).hash_code()) hash_type;

static_assert(sizeof(hash_type) == sizeof(std::size_t), "typeid hash_code type");

const char * typeid_decltype_member_anchor()
{
  return typeid(int).name();
}

static_assert(sizeof(&typeid_decltype_member_anchor) > 0, "typeid body anchor");
