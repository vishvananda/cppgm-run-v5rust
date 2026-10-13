int first, second;
int function(int);
constexpr bool null_equal = nullptr == nullptr;
constexpr bool same = &first == &first;
constexpr bool different = &first != &second;
constexpr bool function_equal = &function == &function;
constexpr bool implicit = &first;
constexpr bool function_implicit = function;
constexpr bool truth = nullptr || &first;
constexpr int* selected = &first ? &first : nullptr;
int result()
{
  return null_equal + same + different + function_equal + implicit +
         function_implicit + truth + (selected == &first);
}
