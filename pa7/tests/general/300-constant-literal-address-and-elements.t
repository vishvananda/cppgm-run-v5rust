constexpr const char* text = "abc";
constexpr bool same = text + 1 == "abc" + 1;
constexpr long distance = (text + 3) - text;
constexpr int element = *(text + 1);
const char& reference = "abc"[2];
constexpr int referred = reference;
int result()
{
  return same + distance + element + referred +
         __builtin_constant_p(text + 4) + __builtin_constant_p(text + 5);
}
