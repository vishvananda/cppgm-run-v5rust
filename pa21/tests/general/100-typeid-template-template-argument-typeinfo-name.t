namespace std {
class type_info {
public:
  bool operator==(const type_info &) const;
  bool operator!=(const type_info &) const;
};
}

namespace nlohmann {
inline namespace json_abi_v3_11_3 {
template<class T> class allocator {};
template<class T, class... Args> class vector {};
template<class T, class SFINAE = void> class adl_serializer {};
template<class Key, class T, class IgnoredLess, class Allocator>
struct ordered_map {};

template<
  template<class, class, class, class> class ObjectType = ordered_map,
  template<class, class...> class ArrayType = vector,
  class StringType = char,
  class BooleanType = bool,
  class NumberIntegerType = long long,
  class NumberUnsignedType = unsigned long long,
  class NumberFloatType = double,
  template<class> class AllocatorType = allocator,
  template<class, class = void> class JSONSerializer = adl_serializer,
  class BinaryType = vector<unsigned char>,
  class CustomBaseClass = void>
class basic_json {};

using ordered_json = basic_json<nlohmann::ordered_map>;
}
}

template<class Json>
struct json_encoder {
  virtual ~json_encoder() {}
};

// An exported accessor makes the same typeinfo observable to host object checks.
const std::type_info& encoder_typeinfo()
{
  return typeid(json_encoder<nlohmann::ordered_json>);
}

int main()
{
  return &encoder_typeinfo() == &typeid(json_encoder<nlohmann::ordered_json>) ? 0 : 1;
}
