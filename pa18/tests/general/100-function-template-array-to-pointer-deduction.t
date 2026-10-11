template<typename T>
T first(const T* p) { return *p; }

int main() { return first("x") - 'x'; }
