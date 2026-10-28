struct Temporary { Temporary(); ~Temporary(); };
int* allocate(int count) { return new int[(Temporary(), count)](); }
