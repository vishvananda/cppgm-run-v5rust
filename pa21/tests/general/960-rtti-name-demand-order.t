namespace std { class type_info {}; }
int before() { return &typeid(int*) == &typeid(int*) ? 0 : 1; }
int thrown() { try { throw (int*)0; } catch (int*) { return 0; } }
int after() { return &typeid(int*) == &typeid(int*) ? 0 : 1; }
int main() { return before() + thrown() + after(); }
