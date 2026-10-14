// Frozen implicit_sequence rejects a bound overload set; expect reject
struct S {
    static int get() { return 7; }
    static int get(int value) { return value; }
};
int invoke(int (*)());
int main() { S local; return invoke(local.get); }
