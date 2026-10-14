// expect exit 0
int nested(int value) {
    goto target;
    if (false) {
        int result;
    target:
        result = value + 2;
        return result;
    }
}
int main() {
    int count = 0;
again:
    int value = ++count;
    if (value < 3) goto again;
    {
        int discarded = 9;
        if (discarded) goto outside;
    }
outside:
    switch (count) {
    case 3:
        goto inner;
    default:
        {
        inner:
            int result = nested(count);
            return result == 5 ? 0 : 1;
        }
    }
}
