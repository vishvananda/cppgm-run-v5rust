struct Constants { static const int value = 3; };
int calls;
Constants object{};
Constants* fetch() { ++calls; return &object; }
long qualified() { return Constants::value; }
long selected() { return fetch()->value; }
int main() { return qualified() + selected() + calls; }
