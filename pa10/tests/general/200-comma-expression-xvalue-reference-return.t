// VALIDATION: compile-pass
int&& move_value(int& value) { return static_cast<int&&>(value); }
int&& get(int& value) { return (void)0, move_value(value); }
int main() {}
