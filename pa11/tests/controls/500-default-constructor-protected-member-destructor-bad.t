// expect reject
struct Base { Base() {} protected: ~Base() {} };
struct Owner : Base { Base member; ~Owner(); };
void use() { Owner value; }
int main() { return 0; }
