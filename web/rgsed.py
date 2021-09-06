#!/usr/bin/env python3
import argparse
import itertools
import os
import re
import subprocess


def get_ripgrep_changes():
    proc = subprocess.Popen(
        [
            'rg', '--no-heading', '--line-number', py_args.query,
            '--replace', py_args.replacement
        ] + rg_args,
        bufsize=1,
        universal_newlines=True,
        stdout=subprocess.PIPE
    )
    return proc.stdout


def get_changes_by_file():
    return itertools.groupby(
        (re.match('(.*):(\d+):(.*)', line).groups() for line in get_ripgrep_changes()),
        key=lambda x: x[0]
    )


def get_action(filename, line_num, new_text, content):
    if not py_args.ask:
        return True, False
    before = content[max(0, line_num - 7):line_num]
    old_text = content[line_num]
    after = content[line_num + 1:min(line_num + 8, len(content))]
    surround = lambda text, colour: chr(27) + '[' + str(colour) + 'm' + text + chr(27) + '[0m'
    print(filename)
    print()
    print(''.join(' ' + x for x in before), end='')
    print(surround('-' + old_text, 31), end='')
    print(surround('+' + new_text, 32))
    print(''.join(' ' + x for x in after))
    while True:
        print('Accept change? [y/n/e/x/q/?] ', end='')
        choice = input()
        if not choice:
            continue
        choice = choice[0].lower()
        if choice == 'y':
            return True, False
        elif choice == 'n':
            return False, False
        elif choice == 'e':
            return False, True
        elif choice == 'x':
            return True, True
        elif choice == 'q':
            raise StopIteration
        print('y - yes')
        print('n - no')
        print('e - edit (batched by file)')
        print('x - yes, edit (batched by file)')
        print('q - quit')


def run():
    for filename, changes in get_changes_by_file():
        with open(filename) as f:
            content = f.readlines()
        try:
            to_be_edited = []
            for _, line_num, new_text in changes:
                line_num = int(line_num) - 1
                accept, edit = get_action(filename, line_num, new_text, content)
                if accept:
                    content[line_num] = new_text + '\n'
                if edit:
                    to_be_edited.append(line_num + 1)
        except:
            raise
        finally:
            with open(filename, 'w') as f:
                f.writelines(content)
        for line_num in to_be_edited:
            subprocess.call([os.environ.get('EDITOR', 'vim'), '+%s' % line_num, filename])


if __name__ == '__main__':
    try:
        parser = argparse.ArgumentParser(prog='rg-sed', allow_abbrev=False)
        parser.add_argument(
            '--ask', action='store_true',
            help='Ask for each replacement.'
        )
        parser.add_argument('query')
        parser.add_argument('replacement')
        # we forward any unrecognised arguments to ripgrep
        py_args, rg_args = parser.parse_known_args()
        run()
    except (StopIteration, KeyboardInterrupt):
        pass
