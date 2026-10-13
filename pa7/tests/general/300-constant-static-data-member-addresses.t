struct object_type { static int value; };
object_type object;
constexpr int* pointer = &((object_type*)0)->value;
constexpr bool same = pointer == &object.value;
int result()
{
  return same;
}
