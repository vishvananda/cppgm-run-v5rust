void first(int&);
void second(int&);

void (*functions[])(int&) = {first, second};
