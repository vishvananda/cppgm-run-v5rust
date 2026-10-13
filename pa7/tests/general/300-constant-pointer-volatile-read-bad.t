int object;
int*volatile pointer = &object;
constexpr bool value = pointer == &object;
