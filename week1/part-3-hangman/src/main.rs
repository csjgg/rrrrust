// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn getinput() -> String {
    print!("Please guess a letter: ");
    // Make sure the prompt from the previous line gets displayed:
    io::stdout().flush().expect("Error flushing stdout.");
    let mut guess = String::new();
    io::stdin()
        .read_line(&mut guess)
        .expect("Error reading line.");
    guess
}

fn checkandmodify(
    nowword: &mut Vec<char>,
    guessed_letters: &mut Vec<char>,
    secret_word: &Vec<char>,
    guess: &String,
) -> i32 {
    let ch = guess.chars().next().unwrap();
    guessed_letters.push(ch);
    let mut i = 0;
    while i < nowword.len() {
        if nowword[i] == '-' && secret_word[i] == ch {
            nowword[i] = ch;
            break;
        } else {
            i += 1;
        }
    }
    if i == nowword.len() {
        1
    } else {
        0
    }
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    // println!("random word: {}", secret_word);

    // Your code here! :)
    let mut wrongtimes = NUM_INCORRECT_GUESSES;
    let mut nowword: Vec<char> = vec!['-'; secret_word_chars.len()];
    let mut guessed_letters: Vec<char> = vec![];
    let mut righttimes = 0;
    print!("Welcome to CS110L Hangman!\n");
    loop {
        println!("The word so far is {}", nowword.iter().collect::<String>());
        println!(
            "You have guessed the following letters:{}",
            guessed_letters.iter().collect::<String>()
        );
        println!("You have {} guesses left", wrongtimes);
        let guess = getinput();
        let i = checkandmodify(
            &mut nowword,
            &mut guessed_letters,
            &secret_word_chars,
            &guess,
        );
        if i == 1 {
            print!("Sorry, that letter is not in the word\n");
            wrongtimes -= 1;
            if wrongtimes == 0 {
                print!("\nSorry, you ran out of guesses!\n");
                break;
            }
        } else {
            righttimes += 1;
            if righttimes == secret_word.len() {
                print!("\nCongratulations you guessed the secret word: {}!\n",nowword.iter().collect::<String>());
                break;
            }
        }
    }
}
