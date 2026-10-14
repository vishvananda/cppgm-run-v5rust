// expect exit 0
struct Text { char text[3]; int value; };
int main() {
    Text bare[2] = {{"hi", 1}, {"ab", 2}};
    Text braced[2] = {{{"cd"}, 3}, {{"ef"}, 4}};
    return bare[0].text[0] == 'h' && bare[1].text[2] == 0 &&
        braced[1].text[1] == 'f' && bare[1].value + braced[1].value == 6 ? 0 : 1;
}
