---
title: torus.cpp
layout: post
author: Luis Diaz
tags: [C++, Computer Graphics]
miniature: https://user-images.githubusercontent.com/41093870/236719650-20d13da5-8467-4892-9515-c463bc9fa52e.gif
description: In this article I explain in great detail my own implementation of Andy Sloane's <strong>donut.c </strong>. Here I go over the maths, code, and everything you need to know to implement a basic terminal render. 
language: en
repo: https://github.com/LDiazN/torus.cpp
es_version: true
---


<br>
![A spinning torus on a terminal, using ASCII art](/assets/images/torus-cpp/video_2023-05-07_22-19-17.gif)

This program is inspired by [donut.c](https://www.a1k0n.net/2011/07/20/donut-math.html), a C program written by **Andy Sloane**. Its goal was to render a spinning donut randomly in space in the terminal using obfuscated C, even its code is in the shape of a donut:

```
             k;double sin()
         ,cos();main(){float A=
       0,B=0,i,j,z[1760];char b[
     1760];printf("\x1b[2J");for(;;
  ){memset(b,32,1760);memset(z,0,7040)
  ;for(j=0;6.28>j;j+=0.07)for(i=0;6.28
 >i;i+=0.02){float c=sin(i),d=cos(j),e=
 sin(A),f=sin(j),g=cos(A),h=d+2,D=1/(c*
 h*e+f*g+5),l=cos      (i),m=cos(B),n=s\
in(B),t=c*h*g-f*        e;int x=40+30*D*
(l*h*m-t*n),y=            12+15*D*(l*h*n
+t*m),o=x+80*y,          N=8*((f*e-c*d*g
 )*m-c*d*e-f*g-l        *d*n);if(22>y&&
 y>0&&x>0&&80>x&&D>z[o]){z[o]=D;;;b[o]=
 ".,-~:;=!*#$@"[N>0?N:0];}}/*#****!!-*/
  printf("\x1b[H");for(k=0;1761>k;k++)
   putchar(k%80?b[k]:10);A+=0.04;B+=
     0.02;}}/*****####*******!!=;:~
       ~::==!!!**********!!!==::-
         .,~~;;;========;;;:~-.
             ..,--------,*/
```

**donut.c** has a special place in my heart because I came across it when I was just starting to learn how to code, and at the time it felt like magic. Now that time has passed and I know much more than I did before, I want to write this article as the explanation I would have liked to have when I saw it for the first time.

Another interesting property of **donut.c** is that it represents the minimal viable version of the graphics rendering pipeline. It is implemented on a reduced version of what we use to implement a render. And the best part is that it can be implemented using a single file, without the need for any additional dependencies beyond the C math library!

In this article, we will step-by-step implement a more readable version of donut.c, this time in **C++**. Following the spirit of donut.c, we will have the following goals:

1. The code should be in a **single file**, `torus.cpp`.
2. The code should be as **simple as possible**, using only  functions and basic control structures.
3. **No additional imports** will be used, beyond some standard C++ libraries for printing, timing, and mathematical calculations.
4. For educational purposes, our program will be more focused on **readability** than obfuscation, unlike Sloane's program.
5. Finally, we will implement a rotating donut (torus or toroid) in the terminal!

# Drawing in a terminal


The idea is to create a program that infinitely runs by replacing a character matrix of size $R$ in the terminal with a small variation of itself. However, a terminal is usually intended to print text continuously, leaving a record of the text that has been printed so far, unlike windows where a graphical interface is drawn and updated each iteration of the program. For this reason, the first thing we need to do is find a way to update a set of cells in a terminal instead of continuously stacking them. This varies a bit depending on the terminal emulator you use, but it follows more or less the same pattern in all of them: **ANSI escape codes**.

ANSI escape codes are used to instruct a terminal emulator to perform a specific action, such as printing characters in a certain color or font, or moving the cursor, which is the case we are interested right now.

We will use ANSI escape codes to instruct the terminal emulator to move the cursor. The cursor is the blinking white bar that marks the position of the next character to be entered. It is usually always at the end of the most recent text, but using ANSI escape codes we can tell it to move to the position we need.

## More about ANSI escape codes

To learn more about ANSI escape codes, you can find a good explanation in [this article](https://notes.burke.libbey.me/ansi-escape-codes/). For now, we only need to know that the code we need has the following format `\033[<some_number>A`, where:

1. The `\033[` character indicates that we are calling a function.
2. The `A` character is the name of the function, which in this case means "move the cursor up a number of rows".
3. The `<some_number>` character is the argument of the function, which in this case represents the number of rows we want to jump.

Now, how can we use this? We want to print and replace a matrix of size $R \times R$, this can be done with the following loop:

```cpp
// Prints continuously as often as possible
while(true)
{
    // Prints the characters of this line
    for(int i = 0; i < R; i++)
    {
        for(int j = 0; j < R; j++)
        {
            cout << "#";
            // In some terminals, there may be more space between rows than between
            // columns of text. For these cases, we can add a space in the form of
            // padding between columns.
            // cout << " ";
        }
    // Prints the newline character that marks the end of the line
    cout << endl; 
  }

  // This is necessary to force the terminal to print what it has in
  // buffer
  cout << flush;
}
```

At the end of the double `for` loop, we will have printed `R` rows to the terminal, and the cursor will be on row `R+1`, because the last row also includes a newline character. Moreover, it will be at the **beginning** of row `R+1`, so to return to the original position, we only need to move the cursor `R` rows up, by printing the code `\033[RA` with `cout`:

```cpp
while(true)
{
    // ... nested for loop
    cout << flush;
    cout << "\033[" << R << "A";
}
```

This is easier to visualize with an example:

![Cursor movement example in a terminal](/assets/images/torus-cpp//Ejemplo_mover_cursor_en_terminal.jpg)

Now that we have a consistent area where we can draw, we still need two things to start working on rendering:

There are two important considerations we need to address before we can implement our drawing loop:

1. **A time interval:** Currently, our program is drawing as often as it can, but this can be very processor-intensive. We would like to impose a limit on the number of "frames" per second. Additionally, we will need the elapsed time for operations that depend on time, such as rotation.
2. **Where to store the drawing:** Since we want to draw a different value for each pixel, we need a matrix to store these data to send them to the drawing function.

## Time interval

Now, we need to tell the program how often we want it to execute. To do this, we will put our program to sleep for a specified amount of time at the end of each iteration. To do this, we need to include `thread` and `chrono`. The first gives us access to the function we will use to put our thread to sleep, and the second allows us to work with time intervals. We can do this by adding the following ***includes*** to the beginning of our program:

```cpp
#include <chrono>
#include <thread>
```

Tu put the current thread (the **main thread**) to sleep, we will use the following line:

```cpp
this_thread::sleep_for(chrono::milliseconds(30));
```

* `this_thread::sleep_for(X)` is used to put the **main thread** to sleep for a specified amount of time.
* `chrono::milliseconds(X)` represents an interval of X milliseconds.
* We choose **30** milliseconds because if we want 30 FPS, then each iteration should last approximately 0.030 (1/30) seconds or 30 milliseconds. To simplify this program, we will assume that the main loop is always instantaneous.
* This line is added at the end of the ***main loop*** to ensure that the program waits a bit after printing the current frame.

Later, we will need a way to know how much time has passed since the start of the application to be able to perform operations that depend on time. For this reason, we will also use a variable to accumulate the time that has elapsed since the start. Consider the following code fragment:

```cpp
// Main loop
auto frame_start = chrono::high_resolution_clock::now();
float time_passed = 0;
while (true)
{
    // Compute the time difference between the start of the previous frame and the current frame,
    // then add it `time_pass`
    auto now = chrono::high_resolution_clock::now();
    auto delta_time = chrono::duration_cast<chrono::milliseconds>(now - frame_start).count() / 1000.0f;
    time_passed += delta_time;
    frame_start = now;
    
    // ... The rest of the main loop
} 
```

* `frame_start` is an object that represents a point in time. In particular, it represents the start of the frame that is currently being processed.
* `time_passed` is the amount of time that has passed since the start of the program in **seconds**
* When the loop starts, we store the current time in `now`, and then we calculate the difference in time between `now` and the last time the loop started, `frame_start`. We add this difference to `time_passed`.
* The line `chrono::duration_cast<chrono::milliseconds>(now - frame_start).count() / 1000.0f;` is simply counting the number of milliseconds between `now` and `frame_start`, and converting it to seconds.

We will now have the amount of time that has passed since the start of the program in `time_passed`. At the end of each iteration, our program will wait a bit for the next frame.

## Canvas

Right now we are printing the same character continuously in the terminal, but we want the character to **depend on the position** in the area where we are drawing. We will use a character matrix of size $R \times R$ where we will store the characters that we will draw in the terminal. We will simply create a matrix `char canvas[R][R]`  whose content will be drawn in the terminal:

```cpp
while(true)
{
    // ... Compute time ...
    for (int i = 0; i < R; i++)
        for (int j = 0; j < R; j++)
            cout << canvas[i][j] << " ";
    // ... The rest of the main loop ... 
}
```

The additional whitespace that is printed after the character is not necessary, but I personally like it, it gives a cleaner finish to the resulting image. If you test the program as it is so far, you may see a matrix of meaningless characters. This is normal, and it is due to the fact that, depending on where you declared the `canvas` matrix, its memory may be uninitialized. You can also add a function to clear the canvas before starting, but we will do this later when we update the canvas.

Until now, you should have the following functionality implemented:

1. We can print to the terminal by replacing the previous image instead of filling the terminal with many repetitions of the same image.
2. There is a time interval between each frame: the iterations of the main loop are not executed immediately.
3. We have a variable that records the time that has passed since the start of the program and is updated in each iteration.
4. The content of the pixels is stored in a matrix of size $R$, the chosen resolution, and is printed from this matrix.

Before proceeding, confirm that everything has gone well so far. Then, we will start working on the drawing of our donut.

# Canvas update

To update the canvas, our first intuition might be to iterate pixel by pixel over the character matrix, assigning a character to each one depending on its position. This approach is the one we would use with  **Ray Tracing,** but we can do it in a simpler way.

Taking advantage of the low resolution of the terminal, our solution will be a simplification of the computer graphics pipeline. The idea will be as follows:

1. We will create all the **points of the donut**, as we would for a 3D model.
2. For each point, we will **transform it** so that the object is visible and has the appearance that we want in our canvas. This stage corresponds to the **Vertex Shader** in the computer graphics pipeline.
3. After transforming it, we will **find the pixel** where this point falls, or rather its position in the canvas matrix. This step corresponds to **rasterization**, but simplified to adapt to the low resolution of the terminal.
4. Then, using the direction of the light and the normal at that point, we will **choose the character** that we will assign to that position of the matrix, coloring this pixel. This step corresponds to **shading or Fragment Shader**.

Before we start, we will setup the function where we will work. The `update_canvas` function will be responsible for updating the character canvas in each iteration of the **main loop.** Since our canvas must be accessible to the **main function** we will pass this matrix by reference to the update function. Thus, the initial version is as follows:

```cpp
void update_canvas(char (&canvas)[R][R])
{
   // TODO: Actually update canvas
}
```



>The type of the parameter `canvas` may seem a bit strange, but the parentheses simply indicate that the parameter is a **character matrix** passed by reference, instead of an array of character references, which would generate a compilation error. It is important to pass this array by reference because otherwise, the entire contents of the array would be copied into the function arguments and this can be large. In addition, the external array would not be modified, so the resulting array would have to be returned by value, which would imply another copy.

We will add this function to our ***main loop***, just before printing the characters to the terminal.

```cpp
while(true)
{
    // ... Compute time ...
    update_canvas(canvas);
    for (int i = 0; i < R; i++)
        for (int j = 0; j < R; j++)
            cout << canvas[i][j] << " ";
    // ... The rest of the main loop ... 
} 
```

Now that we have our `update_canvas` function in place, we can start working on implementing it.

## Generating vertices

Before we start, we will clean up the canvas assigning a whitespace character `' '` to each cell, this way it will be "blank". We need to do this because we will only assing a color to a cell if it corresponds to a torus point. If there's no torus point for that cell, its content will be unmodified. 

```cpp
void update_canvas(char (&canvas)[R][R])
{
    // Clean up the canvas assigning a  
    // whitespace character to each position
   for(int i = 0; i < R; i++)
        for(int j = 0; j < R; j++)
            canvas[i][j] = ' ';
}
```

The next step is to **generate torus points.** Normally, we would load a model from a file, but the torus is a **parametric shape,** and therefore we can use a function to generate the vertices. We will use the following function:

$$
P(\theta, \phi) = 
\begin{cases}
(r_a + r_bcos(\theta)) cos(\phi)\\
(r_a + r_bcos(\theta)) sin(\phi)\\
-r_b\ cos(\theta)
\end{cases} \\ \\
0\le \theta \le 2 \pi \\
0\le \phi \le 2 \pi \\
$$

Where:

- \$\color{red}{r_a}\$ is the radius of the circumference of the torus
- \$\color{blue}{r_b}\$ is the radius of the “tube”, the circle that is rotated to obtain the torus, which is a solid of revolution.
- \$\color{blue}\theta, \color{red}\phi\$ are angles used to parameterize the torus shape. The angle \$ \color{blue} \theta\$ indicates where we are on the outer circle, while the angle \$ \color{red} \phi\$ indicates the position on the inner circle.

![Los parámetros **Ra, Rb**,  corresponden a los radios del círculo y el tubo respectivamente, los ejes son **X**, **Y**, **Z**](/assets/images/torus-cpp/image_2023-06-06_201607213.png)

The parameters $\color{red}{R_a}$, $\color{blue}{R_b}$ correspond to the radii of the circle and the tube, respectively. The axes are $\color{red}X$, $\color{green}Y$, $\color{blue}Z$.

![El ángulo **θ** corresponde al ángulo en la circunferencia del tubo, y el ángulo **Φ** corresponde al ángulo en la circunferencia interna.](/assets/images/torus-cpp/image_2023-06-07_201159283.png)

The angle $\color{blue}{\theta}$ corresponds to the angle on the circumference of the tube, and the angle $\color{red}{\phi}$ corresponds to the angle on the inner circumference.

To represent this parametric equation for a torus, we will use the following function:

```cpp
void torus_point(float theta, float phi, float torus_radius, float tube_radius, float &out_x, float &out_y, float &out_z)
{
    out_x = (torus_radius + tube_radius * cos(theta)) * cos(phi);
    out_y = (torus_radius + tube_radius * cos(theta)) * sin(phi);
    out_z = -tube_radius * sin(theta);
}
```

To use `cos` and `sin` we will need to include `math`, with `#include <math>`.

This function computes the three coordinates of a torus point using the equation we showed earlier. Note that, since we will return three values, the variables where the results are stored are passed by reference. As an additional exercise, you could implement a `struct` that represents a point or vector. In this article, we wont do this to limit ourselves to  variables, functions, and basic control structures.

The torus is a **continuous surface**, which meanas that it has infinite points, but we only need a few.