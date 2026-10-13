static union { int first; int second; };
struct object_type { int prefix; int array[3]; };
object_type object;
constexpr bool aliases = &first == &second;
constexpr bool element = object.array + 2 == &object.array[2];
int result()
{
  return aliases + element;
}
