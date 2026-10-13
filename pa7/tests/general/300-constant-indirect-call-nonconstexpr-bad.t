int function(int);
constexpr int (*pointer)(int) = function;
constexpr int value = pointer(1);
