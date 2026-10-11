class result;
result make_result();

typedef result (*function_pointer)();
function_pointer stored = &make_result;
