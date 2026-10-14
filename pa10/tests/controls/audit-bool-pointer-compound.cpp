// expect exit 0
int numbers[8];
char bytes[8];
bool flag;
int visits;
bool& left() { ++visits; return flag; }
int* right() { ++visits; return numbers; }
int main() {
    bool a = false;
    a += numbers + 1;
    bool b = true;
    b += bytes;
    volatile bool c = false;
    c += numbers;
    left() += right();
    return a && b && c && flag && visits == 2 ? 0 : 1;
}
