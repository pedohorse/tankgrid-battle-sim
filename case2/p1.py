while True:
    looked = look('forward')
    if (l := looked[-1][1]):
        print(f'see: {l}')
        if l.startswith('player'):
            shoot()
            turn_cw()
            move_forward()
            continue
        elif l.startswith('ammocrate'):
            for i in range(len(looked)):
                move_forward()
            continue

    need_to_turn = len(looked) == 1  # we facing sight blocked

    looked_l = look('left')
    looked_r = look('right')
    if (l := looked_l[-1][1]):
        if l.startswith('player'):
            turn_ccw()
            shoot()
            need_to_turn = False
            continue
        elif l.startswith('ammocrate'):
            turn_ccw()
            need_to_turn = False
            continue
    if (l := looked_r[-1][1]):
        if l.startswith('player'):
            turn_cw()
            shoot()
            need_to_turn = False
            continue
        elif l.startswith('ammocrate'):
            turn_cw()
            need_to_turn = False
            continue
    if need_to_turn and (len(looked_l) > 1 or len(looked_r) > 1):
        if len(looked_l) > len(looked_r):
            turn_ccw()
        else:
            turn_cw()
        continue
    elif need_to_turn:  # deadend!
        turn_cw()
        turn_cw()
    move_forward()

