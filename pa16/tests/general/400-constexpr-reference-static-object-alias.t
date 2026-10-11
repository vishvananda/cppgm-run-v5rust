int object;
constexpr int& reference = object;
int& second = reference;
static_assert(&reference == &object, "constant address of static object");
constexpr const int& temporary = 3;
constexpr const int& alias = temporary;
static_assert(temporary == 3, "read lifetime-extended temporary");
static_assert(&temporary == &alias, "aliases share temporary storage");

int main()
{
  second = 7;
  return &reference == &object && &second == &object && object == 7 &&
         temporary == 3 && &temporary == &alias ? 0 : 1;
}
