namespace std {
enum _Ios_Iostate { goodbit = 0, badbit = 1 };
}

namespace abi_link {
int std_enum_probe(std::_Ios_Iostate state)
{
  return state == std::badbit ? 5 : 9;
}
}
