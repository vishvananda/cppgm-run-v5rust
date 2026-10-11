struct value { const char* name; int (*next)(int); };

value make(int n) { switch (n) { default: return value{}; } }

int main() { value v = make(1); return v.name == nullptr ? 0 : 1; }
