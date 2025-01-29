# scan-renamer

This is a small utility that is part of my document management process. When
scanning paper documents, they first run through an OCR pipeline and are then
stored in a directory on my file server. The problem is that the filenames will
be all generic and non-descriptive, requiring manual sorting.

This is where this tool comes in: It offers a website which shows the PDF and
let's you sort it into some directory. It will also attempt to find a valid date
in the OCR'ed document.

I'm thinking about making it smarter with by teaching it where to sort what -
but then there's also [a good reason against that](https://xkcd.com/1319/).

## Audience

This is basically just for me - be inspired or use it, but don't bother with
improvements etc.

## Quality

Note that the code quality is horrible - unwraps everywhere. But I really needed
to sort a lot of documents at once and that was more important than getting the
code to not panic.

Also, it requires `pdftotext` in the path.

## License

This work is licensed under the MIT license. I did not bother to include a
LICENSE file. If this isn't clear enough already, then you're not allowed to use
it.
