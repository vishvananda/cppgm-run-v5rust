struct bits { unsigned value : 3; };
constexpr bits object = {2};
constexpr const unsigned *pointer = __builtin_addressof(object.value);
