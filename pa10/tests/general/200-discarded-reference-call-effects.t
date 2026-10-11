volatile int value = 7;
int calls;

volatile int& get_reference() {
    ++calls;
    return value;
}

int main() {
    (void)get_reference();
    get_reference();
    static_cast<void>(get_reference());
    // The id expression and the used reference result both require a read.
    (void)value;
    int observed = get_reference();
    return calls != 4 || observed != 7;
}
