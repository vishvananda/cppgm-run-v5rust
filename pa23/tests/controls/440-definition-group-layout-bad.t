// External virtual definitions index completed groups, including groups
// whose key function is defined in another unit; expect reject
struct First { virtual void first(); };
struct Second { virtual void second(); };
struct Group : First, Second {
    virtual void external_key();
    char huge[1152921504606846976];
};
void First::first() {}
int main() { return 0; }
