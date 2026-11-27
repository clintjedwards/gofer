# VHS documentation
#
# Output:
#   Output <path>.gif               Create a GIF output at the given <path>
#   Output <path>.mp4               Create an MP4 output at the given <path>
#   Output <path>.webm              Create a WebM output at the given <path>
#
# Settings:
#   Set FontSize <number>           Set the font size of the terminal
#   Set FontFamily <string>         Set the font family of the terminal
#   Set Height <number>             Set the height of the terminal
#   Set Width <number>              Set the width of the terminal
#   Set LetterSpacing <float>       Set the font letter spacing (tracking)
#   Set LineHeight <float>          Set the font line height
#   Set Theme <string>              Set the theme of the terminal (JSON)
#   Set Padding <number>            Set the padding of the terminal
#   Set Framerate <number>          Set the framerate of the recording
#   Set PlaybackSpeed <float>       Set the playback speed of the recording
#
# Sleep:
#   Sleep <time>                    Sleep for a set amount of <time> in seconds
#
# Type:
#   Type[@<time>] "<characters>"    Type <characters> into the terminal with a
#                                   <time> delay between each character
#
# Keys:
#   Backspace[@<time>] [number]     Press the Backspace key
#   Down[@<time>] [number]          Press the Down key
#   Enter[@<time>] [number]         Press the Enter key
#   Space[@<time>] [number]         Press the Space key
#   Tab[@<time>] [number]           Press the Tab key
#   Left[@<time>] [number]          Press the Left Arrow key
#   Right[@<time>] [number]         Press the Right Arrow key
#   Up[@<time>] [number]            Press the Up Arrow key
#   Down[@<time>] [number]          Press the Down Arrow key
#   Ctrl+<key>                      Press the Control key + <key> (e.g. Ctrl+C)
#
# Display:
#   Hide                            Hide the subsequent commands from the output
#   Show                            Show the subsequent commands in the output


Output demo.mp4

Set FontSize 18
Set Width 1640
Set Height 1300
Set Padding 20
Set TypingSpeed 60ms
Set Theme { "name": "Eastwood", "black": "#424242", "red": "#ff7675", "green": "#badc58", "yellow": "#ffeaa7", "blue": "#74b9ff", "magenta": "#cc99cc", "cyan": "#81ecec", "white": "#d3d0c8", "brightBlack": "#c4c2bb", "brightRed": "#ff7675", "brightGreen": "#badc58", "brightYellow": "#ffeaa7", "brightBlue": "#74b9ff", "brightMagenta": "#cc99cc", "brightCyan": "#81ecec", "brightWhite": "#d3d0c8", "background": "#424242", "foreground": "#d3d0c8", "selection": "#74b9ff", "cursor": "#d3d0c8" }

Hide
Type "export PATH=$PWD/../../../../target/debug:$PATH BAT_THEME=ansi BAT_STYLE=plain BAT_PAGING=never BAT_TABS=4 && cd ../../../../examplePipelines/go/images && clear"
Enter
Sleep 500ms
Show

Sleep 1s
Type "bat main.go"
Sleep 500ms
Enter
Sleep 14s

Type "clear"
Enter

# Builds every task's image and registers the pipeline with them.
Type "gofer up ."
Sleep 500ms
Enter
Sleep 8s

Type "clear"
Enter

Type "gofer pipeline run images"
Sleep 500ms
Enter
Sleep 3s

Type "clear"
Enter

Type "gofer fetch images 1"
Sleep 500ms
Enter
Sleep 7s

Type "clear"
Enter

Type "gofer task logs images 1 report"
Sleep 500ms
Enter
Sleep 3s

Type "gofer task logs images 1 cleanup"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Enter

# Change one file, and only the task that copies it in is built again.
Type `echo 'print("cleanup: and one more line")' >> cleanup.py`
Sleep 500ms
Enter
Sleep 1s

Type "gofer up ."
Sleep 500ms
Enter
Sleep 4s
Type "y"
Sleep 500ms
Enter
Sleep 7s

Type "clear"
Enter

Type "gofer run debug images 1"
Sleep 500ms
Enter
Sleep 12s

Type "clear"
Enter

Type "gofer extension list"
Sleep 500ms
Enter
Sleep 6s

# Puts the example back the way it was.
Hide
Type "git checkout -- cleanup.py"
Enter
Sleep 500ms
